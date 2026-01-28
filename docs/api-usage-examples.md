# PROXY Protocol v2 API 使用示例

**版本**: 0.2.0
**日期**: 2026-01-28

---

## API 概览

库提供两种解析模式：

| 函数 | 返回类型 | 非 PROXY 协议行为 | 使用场景 |
|------|---------|-----------------|---------|
| `parse_proxy_protocol` | `Result<ProxyProtocol>` | 返回 `Ok(ProxyProtocol::Unknown)` | **可选 PROXY 协议** |
| `require_proxy_protocol` | `Result<ProxyHeader>` | 返回 `Err(InvalidSignature)` | **强制 PROXY 协议** |

---

## 场景 A：可选 PROXY 协议

**使用场景**：同一端口既接受 PROXY 连接也接受普通连接。

### 示例代码

```rust
use tokio::net::{TcpListener, TcpStream};
use proxy_protocol::version2::{parse_proxy_protocol, ProxyProtocol};

async fn handle_connection(mut stream: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    // 尝试解析 PROXY 协议
    match parse_proxy_protocol(&mut stream).await? {
        ProxyProtocol::V2(header) => {
            // 这是一个 PROXY 连接
            if let Some(addrs) = header.addresses {
                println!("PROXY connection from: {}", addrs.source);
                println!("  → Original destination: {}", addrs.destination);

                // 使用真实客户端地址进行后续处理
                handle_proxied_request(stream, addrs.source).await?;
            } else {
                // LOCAL 命令（健康检查）
                println!("PROXY LOCAL command (health check)");
                handle_health_check(stream).await?;
            }
        }
        ProxyProtocol::Unknown => {
            // 这是一个普通的直连
            println!("Direct connection (no PROXY protocol)");

            // 使用 stream 的 peer_addr 作为客户端地址
            let peer_addr = stream.peer_addr()?;
            handle_direct_request(stream, peer_addr).await?;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    println!("Server listening on 0.0.0.0:8080 (PROXY protocol optional)");

    loop {
        let (stream, _) = listener.accept().await?;
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream).await {
                eprintln!("Connection error: {}", e);
            }
        });
    }
}

async fn handle_proxied_request(
    stream: TcpStream,
    real_client: std::net::SocketAddr
) -> Result<(), Box<dyn std::error::Error>> {
    // 处理代理连接的逻辑
    println!("Processing proxied request from {}", real_client);
    Ok(())
}

async fn handle_direct_request(
    stream: TcpStream,
    client: std::net::SocketAddr
) -> Result<(), Box<dyn std::error::Error>> {
    // 处理直连的逻辑
    println!("Processing direct request from {}", client);
    Ok(())
}

async fn handle_health_check(stream: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    // 处理健康检查
    println!("Health check OK");
    Ok(())
}
```

---

## 场景 B：强制 PROXY 协议

**使用场景**：专用的 PROXY 端口，所有连接必须是 PROXY 协议，拒绝直连。

### 示例代码

```rust
use tokio::net::{TcpListener, TcpStream};
use proxy_protocol::version2::require_proxy_protocol;

async fn handle_proxy_only_connection(
    mut stream: TcpStream
) -> Result<(), Box<dyn std::error::Error>> {
    // 强制要求 PROXY 协议，如果不是则返回错误
    let header = require_proxy_protocol(&mut stream).await?;

    if let Some(addrs) = header.addresses {
        println!("✓ Valid PROXY connection from: {}", addrs.source);
        println!("  → Destination: {}", addrs.destination);

        // 使用真实客户端地址处理请求
        process_request(stream, addrs).await?;
    } else {
        // LOCAL 命令
        println!("✓ PROXY LOCAL command (health check)");
        send_health_check_response(stream).await?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("0.0.0.0:8443").await?;
    println!("Server listening on 0.0.0.0:8443 (PROXY protocol required)");

    loop {
        let (stream, peer) = listener.accept().await?;

        tokio::spawn(async move {
            match handle_proxy_only_connection(stream).await {
                Ok(_) => {
                    println!("Connection from {} completed successfully", peer);
                }
                Err(e) => {
                    // 非 PROXY 协议连接会在这里被拒绝
                    eprintln!("✗ Rejected connection from {}: {}", peer, e);
                }
            }
        });
    }
}

async fn process_request(
    stream: TcpStream,
    addrs: proxy_protocol::version2::ProxyAddresses,
) -> Result<(), Box<dyn std::error::Error>> {
    // 业务逻辑
    println!("Processing request from {}", addrs.source);
    Ok(())
}

async fn send_health_check_response(
    stream: TcpStream
) -> Result<(), Box<dyn std::error::Error>> {
    // 健康检查响应
    Ok(())
}
```

---

## 场景 C：混合架构

**使用场景**：一个应用同时监听两个端口：一个可选 PROXY，一个强制 PROXY。

### 示例代码

```rust
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 端口 8080: 可选 PROXY 协议（公共端口）
    let public_listener = TcpListener::bind("0.0.0.0:8080").await?;

    // 端口 8443: 强制 PROXY 协议（内部负载均衡器端口）
    let lb_listener = TcpListener::bind("0.0.0.0:8443").await?;

    println!("Server started:");
    println!("  - Public port 8080: PROXY protocol optional");
    println!("  - LB port 8443: PROXY protocol required");

    tokio::spawn(async move {
        loop {
            let (stream, _) = public_listener.accept().await.unwrap();
            tokio::spawn(handle_optional_proxy(stream));
        }
    });

    loop {
        let (stream, _) = lb_listener.accept().await?;
        tokio::spawn(handle_required_proxy(stream));
    }
}

async fn handle_optional_proxy(stream: tokio::net::TcpStream) {
    // 使用 parse_proxy_protocol（可选）
    use proxy_protocol::version2::{parse_proxy_protocol, ProxyProtocol};

    match parse_proxy_protocol(&mut stream).await {
        Ok(ProxyProtocol::V2(header)) => {
            println!("Public port: PROXY connection");
        }
        Ok(ProxyProtocol::Unknown) => {
            println!("Public port: Direct connection");
        }
        Err(e) => {
            eprintln!("Public port error: {}", e);
        }
    }
}

async fn handle_required_proxy(stream: tokio::net::TcpStream) {
    // 使用 require_proxy_protocol（强制）
    use proxy_protocol::version2::require_proxy_protocol;

    match require_proxy_protocol(&mut stream).await {
        Ok(header) => {
            println!("LB port: Valid PROXY connection");
        }
        Err(e) => {
            eprintln!("LB port: Rejected (not PROXY protocol): {}", e);
        }
    }
}
```

---

## 错误处理

### 常见错误类型

```rust
use proxy_protocol::result::Error;

match require_proxy_protocol(&mut stream).await {
    Ok(header) => {
        // 成功解析
    }
    Err(Error::InvalidSignature) => {
        // 不是 PROXY 协议
        eprintln!("Not a PROXY protocol connection");
    }
    Err(Error::InvalidAddressFamily(family)) => {
        // 不支持的地址族
        eprintln!("Unsupported address family: {:#x}", family);
    }
    Err(Error::AddressLengthMismatch { family, got, expected }) => {
        // 地址数据长度不匹配
        eprintln!("{} address length mismatch: got {}, expected {}",
                  family, got, expected);
    }
    Err(Error::Io(e)) => {
        // I/O 错误
        eprintln!("I/O error: {}", e);
    }
    Err(e) => {
        // 其他错误
        eprintln!("Error: {}", e);
    }
}
```

---

## 性能建议

### 1. 连接池场景

在连接池或长连接场景下，PROXY 协议只需在连接建立时解析一次：

```rust
struct ProxiedConnection {
    stream: TcpStream,
    real_client: SocketAddr,
}

impl ProxiedConnection {
    async fn new(mut stream: TcpStream) -> Result<Self, Box<dyn std::error::Error>> {
        let header = require_proxy_protocol(&mut stream).await?;
        let real_client = header.addresses
            .map(|a| a.source)
            .unwrap_or_else(|| stream.peer_addr().unwrap());

        Ok(Self { stream, real_client })
    }
}
```

### 2. 超时设置

建议为 PROXY 协议解析设置超时：

```rust
use tokio::time::{timeout, Duration};

let result = timeout(
    Duration::from_secs(5),
    require_proxy_protocol(&mut stream)
).await??;
```

### 3. 缓冲优化

对于高吞吐场景，考虑使用 `BufReader`：

```rust
use tokio::io::BufReader;

let mut buffered = BufReader::new(stream);
// 注意：当前实现需要 TcpStream，暂不支持 BufReader
// 未来版本可能支持泛型 AsyncRead
```

---

## 最佳实践

1. **明确语义**：使用 `require_proxy_protocol` 让意图更清晰
2. **错误处理**：区分不同的错误类型，给予适当的响应
3. **日志记录**：记录 PROXY 协议解析结果，便于调试
4. **超时保护**：避免慢速客户端占用资源
5. **监控指标**：统计 PROXY vs 直连的比例

---

**作者**: Claude Code
**审阅**: 待审阅
**状态**: 已完成 ✅
