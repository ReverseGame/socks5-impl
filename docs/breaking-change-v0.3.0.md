# PROXY Protocol v2 Breaking Change - v0.3.0

**日期**: 2026-01-28
**版本**: 0.2.0 → 0.3.0
**类型**: 破坏性变更（Breaking Change）

---

## 变更概述

简化 API 设计，删除 `ProxyProtocol` 枚举，使 `parse_proxy_protocol` 在检测到非 PROXY 协议时直接返回错误。

---

## API 变更

### 之前（v0.2.0）

```rust
pub enum ProxyProtocol {
    V2(ProxyHeader),
    Unknown,
}

pub async fn parse_proxy_protocol(stream: &mut TcpStream) -> Result<ProxyProtocol>
```

**使用方式**：
```rust
match parse_proxy_protocol(&mut stream).await? {
    ProxyProtocol::V2(header) => {
        // 处理 PROXY 协议
    }
    ProxyProtocol::Unknown => {
        // 处理普通连接
    }
}
```

### 现在（v0.3.0）

```rust
// ProxyProtocol 枚举已删除

pub async fn parse_proxy_protocol(stream: &mut TcpStream) -> Result<ProxyHeader>
```

**使用方式**：
```rust
// 直接获取 ProxyHeader，如果不是 PROXY 协议会返回错误
let header = parse_proxy_protocol(&mut stream).await?;

if let Some(addrs) = header.addresses {
    println!("Real client: {}", addrs.source);
}
```

---

## 迁移指南

### 场景 1：强制 PROXY 协议

**之前**：
```rust
match parse_proxy_protocol(&mut stream).await? {
    ProxyProtocol::V2(header) => {
        // 处理 PROXY 连接
        let addrs = header.addresses.unwrap();
        handle_proxy(stream, addrs).await
    }
    ProxyProtocol::Unknown => {
        // 拒绝连接
        return Err("Not a PROXY protocol connection".into());
    }
}
```

**现在**：
```rust
// 更简洁！直接获取 header，失败会自动返回错误
let header = parse_proxy_protocol(&mut stream).await?;

if let Some(addrs) = header.addresses {
    handle_proxy(stream, addrs).await
}
```

---

### 场景 2：可选 PROXY 协议（需要手动处理）

**之前**：
```rust
match parse_proxy_protocol(&mut stream).await? {
    ProxyProtocol::V2(header) => {
        // 处理 PROXY 连接
        let real_client = header.addresses.unwrap().source;
        handle_request(stream, real_client).await
    }
    ProxyProtocol::Unknown => {
        // 处理普通连接
        let peer = stream.peer_addr()?;
        handle_request(stream, peer).await
    }
}
```

**现在**：
```rust
// 需要手动 peek 检查签名（推荐使用辅助函数）
use tokio::io::AsyncReadExt;

// 方案 1：尝试解析，失败则作为普通连接
let client_addr = match parse_proxy_protocol(&mut stream).await {
    Ok(header) => {
        // 这是 PROXY 连接
        header.addresses.map(|a| a.source).unwrap_or_else(|| stream.peer_addr().unwrap())
    }
    Err(_) => {
        // 这是普通连接（解析失败）
        stream.peer_addr()?
    }
};

handle_request(stream, client_addr).await
```

或者使用专用辅助函数：

```rust
// 方案 2：创建辅助函数
async fn try_parse_proxy_protocol(stream: &mut TcpStream) -> Result<Option<ProxyHeader>> {
    // 先 peek 检查签名
    let mut buf = [0u8; 12];
    let n = stream.peek(&mut buf).await?;

    if n >= 12 && &buf == b"\r\n\r\n\x00\r\nQUIT\n" {
        // 可能是 PROXY 协议，尝试解析
        parse_proxy_protocol(stream).await.map(Some)
    } else {
        // 不是 PROXY 协议
        Ok(None)
    }
}

// 使用
match try_parse_proxy_protocol(&mut stream).await? {
    Some(header) => {
        // PROXY 连接
        let addrs = header.addresses.unwrap();
        handle_request(stream, addrs.source).await
    }
    None => {
        // 普通连接
        let peer = stream.peer_addr()?;
        handle_request(stream, peer).await
    }
}
```

---

## 错误处理变更

### 新增错误类型

```rust
Error::InvalidSignature  // 不是 PROXY Protocol v2（签名不匹配）
Error::UnsupportedVersion(u8)  // 版本号不是 2
```

### 错误处理示例

```rust
use proxy_protocol::result::Error;

match parse_proxy_protocol(&mut stream).await {
    Ok(header) => {
        // 成功解析
        println!("PROXY connection");
    }
    Err(Error::InvalidSignature) => {
        // 不是 PROXY 协议连接
        println!("Not a PROXY protocol connection");
    }
    Err(Error::UnsupportedVersion(v)) => {
        // 版本不支持
        eprintln!("Unsupported PROXY protocol version: {}", v);
    }
    Err(e) => {
        // 其他错误
        eprintln!("Error: {}", e);
    }
}
```

---

## 删除的 API

以下 API 已删除：

| API | 状态 | 替代方案 |
|-----|------|---------|
| `ProxyProtocol` 枚举 | ❌ 已删除 | 直接使用 `ProxyHeader` |
| `ProxyProtocol::V2` | ❌ 已删除 | `parse_proxy_protocol` 直接返回 `ProxyHeader` |
| `ProxyProtocol::Unknown` | ❌ 已删除 | 返回 `Err(InvalidSignature)` |
| `require_proxy_protocol` | ❌ 已删除 | 使用 `parse_proxy_protocol`（现在行为相同） |

---

## 迁移检查清单

- [ ] 将所有 `ProxyProtocol::V2(header)` 替换为直接使用 `header`
- [ ] 删除对 `ProxyProtocol::Unknown` 的处理
- [ ] 如果需要可选 PROXY 协议，添加错误处理逻辑
- [ ] 更新错误处理，处理 `InvalidSignature` 和 `UnsupportedVersion`
- [ ] 运行测试确保迁移正确
- [ ] 更新 Cargo.toml 版本到 0.3.0

---

## 优势

### 简化的 API

✅ **代码更简洁**：从 `match` 模式匹配简化为直接 `?` 操作符
✅ **类型更清晰**：直接返回 `Result<ProxyHeader>`，语义更明确
✅ **错误处理统一**：所有失败情况都通过 `Result::Err` 返回

### 性能保持

✅ **系统调用**：仍然是 2 次（peek + read_exact）
✅ **内存分配**：仍然是 1 次（地址数据缓冲区）
✅ **零拷贝**：签名检查和地址解析依然零拷贝

---

## 常见问题

### Q1: 如何支持可选 PROXY 协议？

**A**: 使用错误处理或 peek 检查签名。推荐方案见"场景 2"。

### Q2: 之前的 `require_proxy_protocol` 去哪了？

**A**: 已删除。现在 `parse_proxy_protocol` 的行为就是强制 PROXY 协议。

### Q3: 这次变更会影响性能吗？

**A**: 不会。性能特性完全相同（2次系统调用，1次堆分配，零拷贝）。

### Q4: 如何快速检测是否是 PROXY 协议？

**A**: 使用 `peek` 检查前 12 字节签名：
```rust
let mut buf = [0u8; 12];
stream.peek(&mut buf).await?;
let is_proxy = &buf == b"\r\n\r\n\x00\r\nQUIT\n";
```

---

## 版本兼容性

| 版本 | API | 向后兼容 |
|------|-----|---------|
| v0.1.0 | 初始版本 | - |
| v0.2.0 | 性能优化 + `require_proxy_protocol` | ✅ 兼容 v0.1.0 |
| v0.3.0 | 简化 API（本次变更） | ❌ 破坏性变更 |

---

## 建议

1. **新项目**：直接使用 v0.3.0
2. **现有项目（强制 PROXY）**：迁移很简单，直接更新
3. **现有项目（可选 PROXY）**：需要添加错误处理逻辑

---

**作者**: Claude Code
**审阅**: 待审阅
**状态**: 已完成 ✅
