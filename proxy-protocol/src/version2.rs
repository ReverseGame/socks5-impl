use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use tokio::{io::AsyncReadExt, net::TcpStream};

use crate::result::{Error, Result};

const SIGNATURE_LENGTH: usize = 12;
const PROXY_SIGNATURE: &[u8] = b"\r\n\r\n\x00\r\nQUIT\n";
const HEADER_SIZE: usize = 16; // 签名12 + 版本1 + 协议1 + 长度2

/// 地址族常量
pub const AF_UNSPEC: u8 = 0x00;
pub const AF_INET: u8 = 0x10;
pub const AF_INET6: u8 = 0x20;
pub const AF_UNIX: u8 = 0x30;

/// 协议常量
pub const PROTO_UNSPEC: u8 = 0x00;
pub const PROTO_STREAM: u8 = 0x01;
pub const PROTO_DGRAM: u8 = 0x02;

/// 命令类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// LOCAL 命令：健康检查，无实际代理信息
    Local,
    /// PROXY 命令：包含真实客户端地址
    Proxy,
}

/// PROXY Protocol v2 地址信息
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyAddresses {
    /// 源地址（真实客户端）
    pub source: SocketAddr,
    /// 目标地址（代理接收地址）
    pub destination: SocketAddr,
}

impl ProxyAddresses {
    /// 获取源 IP 地址
    pub fn source_ip(&self) -> IpAddr {
        self.source.ip()
    }

    /// 获取源端口
    pub fn source_port(&self) -> u16 {
        self.source.port()
    }

    /// 获取目标 IP 地址
    pub fn destination_ip(&self) -> IpAddr {
        self.destination.ip()
    }

    /// 获取目标端口
    pub fn destination_port(&self) -> u16 {
        self.destination.port()
    }
}

/// PROXY Protocol v2 头部
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyHeader {
    /// 命令类型
    pub command: Command,
    /// 地址族（AF_INET, AF_INET6 等）
    pub address_family: u8,
    /// 协议类型（PROTO_STREAM, PROTO_DGRAM 等）
    pub protocol: u8,
    /// 地址信息（仅当 command=PROXY 时有效）
    pub addresses: Option<ProxyAddresses>,
}

/// 从 TCP 流中解析 PROXY Protocol v2 头部
///
/// 此函数要求连接**必须是 PROXY Protocol v2**，如果检测到非 PROXY 协议连接，
/// 会返回 `Error::InvalidSignature`。
///
/// # 性能特性
/// - 系统调用：2 次（peek + read_exact）
/// - 堆分配：1 次（地址数据缓冲区）
/// - 零拷贝：签名检查、字节序解析均在栈上完成
///
/// # 错误
/// - `InvalidSignature`: 不是 PROXY Protocol v2 连接
/// - `InvalidCommand`: 未知的命令类型
/// - `InvalidAddressFamily`: 不支持的地址族
/// - `Io`: I/O 错误
///
/// # 示例
/// ```no_run
/// use tokio::net::TcpStream;
/// use proxy_protocol::version2::parse_proxy_protocol;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
///
/// // 如果不是 PROXY 协议，会返回 Err(InvalidSignature)
/// let header = parse_proxy_protocol(&mut stream).await?;
///
/// if let Some(addrs) = header.addresses {
///     println!("Real client: {}", addrs.source);
/// }
/// # Ok(())
/// # }
/// ```
pub async fn parse_proxy_protocol(stream: &mut TcpStream) -> Result<ProxyHeader> {
    // 1. 栈分配固定大小缓冲区（避免堆分配）
    let mut header_buf = [0u8; HEADER_SIZE];

    // 2. 一次性 peek 头部（检查是否是 PROXY 协议）
    let n = stream.peek(&mut header_buf).await?;
    if n < HEADER_SIZE {
        // 数据不足，不是 PROXY 协议
        return Err(Error::InvalidSignature);
    }

    // 3. 快速路径：零拷贝签名检查
    if &header_buf[..SIGNATURE_LENGTH] != PROXY_SIGNATURE {
        return Err(Error::InvalidSignature);
    }

    // 4. 解析版本和命令
    let version_command = header_buf[SIGNATURE_LENGTH];
    let version = version_command >> 4;
    let command_byte = version_command & 0x0f;

    if version != 2 {
        return Err(Error::UnsupportedVersion(version));
    }

    let command = match command_byte {
        0x00 => Command::Local,
        0x01 => Command::Proxy,
        _ => return Err(Error::InvalidCommand(command_byte)),
    };

    // 5. 解析地址族和协议
    let family_protocol = header_buf[SIGNATURE_LENGTH + 1];
    let address_family = family_protocol & 0xf0;
    let protocol = family_protocol & 0x0f;

    // 6. 解析地址数据长度（大端序）
    let addr_len = u16::from_be_bytes([header_buf[SIGNATURE_LENGTH + 2], header_buf[SIGNATURE_LENGTH + 3]]) as usize;

    // 验证地址长度合法性（最大 64KB，实际不会超过几百字节）
    if addr_len > 65535 {
        return Err(Error::InvalidAddressLength(addr_len as u16));
    }

    // 7. 计算总帧长度并一次性读取
    let total_len = HEADER_SIZE + addr_len;
    let mut frame_buf = vec![0u8; total_len];
    stream.read_exact(&mut frame_buf).await?;

    // 8. 解析地址信息（仅当 PROXY 命令时才需要）
    let addresses = if command == Command::Proxy && addr_len > 0 {
        Some(parse_addresses(&frame_buf[HEADER_SIZE..], address_family)?)
    } else {
        None
    };

    Ok(ProxyHeader {
        command,
        address_family,
        protocol,
        addresses,
    })
}

/// 从缓冲区解析地址信息（零拷贝）
fn parse_addresses(buf: &[u8], family: u8) -> Result<ProxyAddresses> {
    match family {
        AF_INET => {
            // IPv4: 源IP(4) + 目标IP(4) + 源端口(2) + 目标端口(2) = 12 字节
            const IPV4_ADDR_SIZE: usize = 12;
            if buf.len() < IPV4_ADDR_SIZE {
                return Err(Error::AddressLengthMismatch {
                    family: "IPv4",
                    got: buf.len(),
                    expected: IPV4_ADDR_SIZE,
                });
            }

            let src_addr = Ipv4Addr::new(buf[0], buf[1], buf[2], buf[3]);
            let dst_addr = Ipv4Addr::new(buf[4], buf[5], buf[6], buf[7]);
            let src_port = u16::from_be_bytes([buf[8], buf[9]]);
            let dst_port = u16::from_be_bytes([buf[10], buf[11]]);

            Ok(ProxyAddresses {
                source: SocketAddr::new(IpAddr::V4(src_addr), src_port),
                destination: SocketAddr::new(IpAddr::V4(dst_addr), dst_port),
            })
        }
        AF_INET6 => {
            // IPv6: 源IP(16) + 目标IP(16) + 源端口(2) + 目标端口(2) = 36 字节
            const IPV6_ADDR_SIZE: usize = 36;
            if buf.len() < IPV6_ADDR_SIZE {
                return Err(Error::AddressLengthMismatch {
                    family: "IPv6",
                    got: buf.len(),
                    expected: IPV6_ADDR_SIZE,
                });
            }

            // 零拷贝：直接从切片构造数组
            let src_addr = Ipv6Addr::from(<[u8; 16]>::try_from(&buf[0..16]).unwrap());
            let dst_addr = Ipv6Addr::from(<[u8; 16]>::try_from(&buf[16..32]).unwrap());
            let src_port = u16::from_be_bytes([buf[32], buf[33]]);
            let dst_port = u16::from_be_bytes([buf[34], buf[35]]);

            Ok(ProxyAddresses {
                source: SocketAddr::new(IpAddr::V6(src_addr), src_port),
                destination: SocketAddr::new(IpAddr::V6(dst_addr), dst_port),
            })
        }
        AF_UNIX => {
            // AF_UNIX 地址：216 字节（108 + 108）
            // 目前不解析具体路径，仅占位
            Err(Error::InvalidAddressFamily(family))
        }
        _ => Err(Error::InvalidAddressFamily(family)),
    }
}
