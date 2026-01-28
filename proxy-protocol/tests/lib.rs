#[cfg(test)]
mod tests {
    use proxy_protocol::version2::{parse_proxy_protocol, Command};
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use tokio::{
        io::AsyncWriteExt,
        net::{TcpListener, TcpStream},
    };

    async fn create_test_stream(data: &[u8]) -> TcpStream {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        // 拷贝数据以满足 'static 生命周期要求
        let data_owned = data.to_vec();
        tokio::spawn(async move {
            let mut stream = TcpStream::connect(&addr).await.unwrap();
            stream.write_all(&data_owned).await.unwrap();
            stream.flush().await.unwrap();
            // 保持连接打开，等待测试完成
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        });

        let (test_stream, _) = listener.accept().await.unwrap();
        test_stream
    }

    #[tokio::test]
    async fn test_parse_proxy_protocol_valid_ipv4() {
        // PROXY v2 帧：IPv4
        // 源地址：192.168.1.100:12345 (0xC0A80164 0x3039)
        // 目标地址：10.0.0.1:80 (0x0A000001 0x0050)
        let data = b"\x0D\x0A\x0D\x0A\x00\x0D\x0A\x51\x55\x49\x54\x0A\x21\x11\x00\x0C\xC0\xA8\x01\x64\x0A\x00\x00\x01\x30\x39\x00\x50";
        let mut stream = create_test_stream(data).await;

        let header = parse_proxy_protocol(&mut stream).await.unwrap();

        assert_eq!(header.command, Command::Proxy);
        let addrs = header.addresses.expect("Should have addresses for PROXY command");
        assert_eq!(addrs.source.ip(), IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)));
        assert_eq!(addrs.source.port(), 12345);
        assert_eq!(addrs.destination.ip(), IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)));
        assert_eq!(addrs.destination.port(), 80);
    }

    #[tokio::test]
    async fn test_parse_proxy_protocol_invalid_signature() {
        let data = b"GET / HTTP/1.1\r\nHost: example.com\r\n\r\n";
        let mut stream = create_test_stream(data).await;

        let result = parse_proxy_protocol(&mut stream).await;
        // 应该返回 InvalidSignature 错误
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            proxy_protocol::result::Error::InvalidSignature
        ));
    }

    #[tokio::test]
    async fn test_parse_proxy_protocol_local_command() {
        // LOCAL 命令（健康检查）：版本2 + 命令0
        let data = b"\x0D\x0A\x0D\x0A\x00\x0D\x0A\x51\x55\x49\x54\x0A\x20\x00\x00\x00";
        let mut stream = create_test_stream(data).await;

        let header = parse_proxy_protocol(&mut stream).await.unwrap();

        assert_eq!(header.command, Command::Local);
        assert!(header.addresses.is_none());
    }

    #[tokio::test]
    async fn test_parse_proxy_protocol_ipv6() {
        // IPv6 测试帧
        // 源地址：2001:db8::1:12345
        // 目标地址：2001:db8::2:80
        let mut data = Vec::new();
        data.extend_from_slice(b"\x0D\x0A\x0D\x0A\x00\x0D\x0A\x51\x55\x49\x54\x0A"); // 签名
        data.push(0x21); // 版本2 + PROXY命令
        data.push(0x21); // AF_INET6 + STREAM
        data.extend_from_slice(&36u16.to_be_bytes()); // 长度36字节

        // 源IP：2001:db8::1
        data.extend_from_slice(&[0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        // 目标IP：2001:db8::2
        data.extend_from_slice(&[0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2]);
        // 源端口：12345
        data.extend_from_slice(&12345u16.to_be_bytes());
        // 目标端口：80
        data.extend_from_slice(&80u16.to_be_bytes());

        let mut stream = create_test_stream(&data).await;

        let header = parse_proxy_protocol(&mut stream).await.unwrap();

        assert_eq!(header.command, Command::Proxy);
        let addrs = header.addresses.expect("Should have addresses");
        assert!(matches!(addrs.source.ip(), IpAddr::V6(_)));
        assert_eq!(addrs.source.port(), 12345);
        assert_eq!(addrs.destination.port(), 80);
    }

    #[tokio::test]
    async fn test_parse_proxy_protocol_short_frame() {
        // 不完整的帧（只有签名）
        let data = b"\x0D\x0A\x0D\x0A\x00\x0D\x0A\x51\x55\x49\x54";
        let mut stream = create_test_stream(data).await;

        let result = parse_proxy_protocol(&mut stream).await;
        // 应该返回错误
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_proxy_addresses_helpers() {
        let src = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), 8080);
        let dst = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 80);

        let addrs = proxy_protocol::version2::ProxyAddresses {
            source: src,
            destination: dst,
        };

        assert_eq!(addrs.source_ip(), IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)));
        assert_eq!(addrs.source_port(), 8080);
        assert_eq!(addrs.destination_ip(), IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)));
        assert_eq!(addrs.destination_port(), 80);
    }
}
