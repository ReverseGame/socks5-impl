# StreamOperation Trait 简化实施计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将 `StreamOperation` 和 `AsyncStreamOperation` 两个 trait 合并为一个统一的 `StreamOperation` trait

**Architecture:** 删除旧的同步 trait，将 AsyncStreamOperation 重命名为 StreamOperation，所有实现从两个 impl 块合并为一个

**Tech Stack:** Rust, async-trait, tokio, bytes

---

## Task 1: 修改 trait 定义

**Files:**
- Modify: `src/protocol/mod.rs:62-99`

**Step 1: 备份当前 trait 定义**

先查看当前的 trait 定义，确认要修改的内容。

Run: `grep -A 40 "pub trait StreamOperation" src/protocol/mod.rs`
Expected: 看到两个 trait 定义

**Step 2: 删除旧的 StreamOperation trait**

删除同步版本的 StreamOperation trait（第 62-82 行）。

```rust
// 删除这部分
pub trait StreamOperation {
    fn retrieve_from_stream<R>(stream: &mut R) -> std::io::Result<Self>
    where
        R: std::io::Read,
        Self: Sized;

    fn write_to_stream<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        let len = self.len();
        let mut buf = bytes::BytesMut::with_capacity(len);
        self.write_to_buf(&mut buf);
        w.write_all(&buf)
    }

    fn write_to_buf<B: bytes::BufMut>(&self, buf: &mut B);

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
```

**Step 3: 修改 AsyncStreamOperation 为新的 StreamOperation**

将 AsyncStreamOperation trait 改名为 StreamOperation，并添加 `write_to_buf` 和 `len` 方法。

```rust
/// SOCKS5 协议流操作 trait（统一序列化和异步 I/O）
#[async_trait::async_trait]
pub trait StreamOperation {
    /// 从异步流中读取并反序列化对象
    async fn retrieve_from_async_stream<R>(r: &mut R) -> std::io::Result<Self>
    where
        R: AsyncRead + Unpin + Send + ?Sized,
        Self: Sized;

    /// 将对象序列化到缓冲区
    fn write_to_buf<B: BufMut>(&self, buf: &mut B);

    /// 返回序列化后的字节长度
    fn len(&self) -> usize;

    /// 判断是否为空
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 将对象序列化并写入异步流（提供默认实现）
    async fn write_to_async_stream<W>(&self, w: &mut W) -> std::io::Result<()>
    where
        W: AsyncWrite + Unpin + Send + ?Sized,
    {
        let mut buf = bytes::BytesMut::with_capacity(self.len());
        self.write_to_buf(&mut buf);
        w.write_all(&buf).await
    }
}
```

**Step 4: 验证语法正确**

Run: `cargo check --lib`
Expected: 编译错误（因为实现文件还没更新），但 trait 定义应该没有语法错误

**Step 5: Commit trait 定义修改**

```bash
git add src/protocol/mod.rs
git commit -m "refactor(protocol): merge StreamOperation and AsyncStreamOperation traits

- Delete old sync StreamOperation trait
- Rename AsyncStreamOperation to StreamOperation
- Add write_to_buf and len methods to new StreamOperation

This is step 1 of trait simplification - trait definition only.
Implementation files will be updated in subsequent commits.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 2: 更新 Address 实现

**Files:**
- Modify: `src/protocol/address.rs:111-228`

**Step 1: 定位 Address 的两个 impl 块**

Run: `grep -n "impl.*StreamOperation.*for Address" src/protocol/address.rs`
Expected: 看到两行（line 111 和 line 189）

**Step 2: 删除旧的 impl StreamOperation 块**

删除第 111-186 行的同步实现（保留 write_to_buf 和 len 方法的内容，稍后会用到）。

**Step 3: 修改 impl AsyncStreamOperation 为 impl StreamOperation**

将第 189 行的 `impl AsyncStreamOperation for Address` 改为 `impl StreamOperation for Address`。

**Step 4: 添加 write_to_buf 和 len 方法到新的 impl 块**

在 `retrieve_from_async_stream` 方法后面，添加从旧 impl 块中提取的方法：

```rust
fn write_to_buf<B: BufMut>(&self, buf: &mut B) {
    match self {
        Self::SocketAddress(SocketAddr::V4(addr)) => {
            buf.put_u8(AddressType::IPv4.into());
            buf.put_slice(&addr.ip().octets());
            buf.put_u16(addr.port());
        }
        Self::SocketAddress(SocketAddr::V6(addr)) => {
            buf.put_u8(AddressType::IPv6.into());
            buf.put_slice(&addr.ip().octets());
            buf.put_u16(addr.port());
        }
        Self::DomainAddress(addr, port) => {
            let addr = addr.as_bytes();
            buf.put_u8(AddressType::Domain.into());
            buf.put_u8(addr.len() as u8);
            buf.put_slice(addr);
            buf.put_u16(*port);
        }
    }
}

fn len(&self) -> usize {
    match self {
        Address::SocketAddress(SocketAddr::V4(_)) => 1 + 4 + 2,
        Address::SocketAddress(SocketAddr::V6(_)) => 1 + 16 + 2,
        Address::DomainAddress(addr, _) => 1 + 1 + addr.len() + 2,
    }
}
```

**Step 5: 验证编译**

Run: `cargo check --lib`
Expected: Address 相关的错误应该消失

**Step 6: 运行 Address 相关测试**

Run: `cargo test --lib address::test_address`
Expected: 测试通过

**Step 7: Commit**

```bash
git add src/protocol/address.rs
git commit -m "refactor(address): merge impl blocks into single StreamOperation

Merged old StreamOperation and AsyncStreamOperation impl blocks.
Now Address has only one impl block with all methods.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 3: 更新 Request 实现

**Files:**
- Modify: `src/protocol/request.rs`

**Step 1: 定位 Request 的两个 impl 块**

Run: `grep -n "impl.*StreamOperation.*for Request" src/protocol/request.rs`
Expected: 看到两个 impl 块的位置

**Step 2: 合并 impl 块**

按照 Task 2 的模式：
1. 删除旧的 `impl StreamOperation` 块（保留 write_to_buf 和 len 内容）
2. 将 `impl AsyncStreamOperation` 改为 `impl StreamOperation`
3. 添加 write_to_buf 和 len 方法到新的 impl 块

**Step 3: 验证编译**

Run: `cargo check --lib`
Expected: Request 相关的错误应该消失

**Step 4: 运行测试**

Run: `cargo test --lib request`
Expected: 测试通过

**Step 5: Commit**

```bash
git add src/protocol/request.rs
git commit -m "refactor(request): merge impl blocks into single StreamOperation

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 4: 更新 Response 实现

**Files:**
- Modify: `src/protocol/response.rs`

**Step 1: 定位 Response 的两个 impl 块**

Run: `grep -n "impl.*StreamOperation.*for Response" src/protocol/response.rs`

**Step 2: 合并 impl 块**

按照相同模式合并。

**Step 3: 验证编译**

Run: `cargo check --lib`

**Step 4: 运行测试**

Run: `cargo test --lib response`

**Step 5: Commit**

```bash
git add src/protocol/response.rs
git commit -m "refactor(response): merge impl blocks into single StreamOperation

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 5: 更新 UdpHeader 实现

**Files:**
- Modify: `src/protocol/udp.rs`

**Step 1: 定位 UdpHeader 的两个 impl 块**

Run: `grep -n "impl.*StreamOperation.*for UdpHeader" src/protocol/udp.rs`

**Step 2: 合并 impl 块**

按照相同模式合并。

**Step 3: 验证编译**

Run: `cargo check --lib`

**Step 4: 运行测试**

Run: `cargo test --lib udp`

**Step 5: Commit**

```bash
git add src/protocol/udp.rs
git commit -m "refactor(udp): merge impl blocks into single StreamOperation

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 6: 更新握手 Request 实现

**Files:**
- Modify: `src/protocol/handshake/request.rs`

**Step 1: 定位两个 impl 块**

Run: `grep -n "impl.*StreamOperation" src/protocol/handshake/request.rs`

**Step 2: 合并 impl 块**

按照相同模式合并。

**Step 3: 验证编译**

Run: `cargo check --lib`

**Step 4: Commit**

```bash
git add src/protocol/handshake/request.rs
git commit -m "refactor(handshake/request): merge impl blocks into single StreamOperation

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 7: 更新握手 Response 实现

**Files:**
- Modify: `src/protocol/handshake/response.rs`

**Step 1: 定位两个 impl 块**

Run: `grep -n "impl.*StreamOperation" src/protocol/handshake/response.rs`

**Step 2: 合并 impl 块**

按照相同模式合并。

**Step 3: 验证编译**

Run: `cargo check --lib`

**Step 4: Commit**

```bash
git add src/protocol/handshake/response.rs
git commit -m "refactor(handshake/response): merge impl blocks into single StreamOperation

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 8: 更新密码认证 Request 实现

**Files:**
- Modify: `src/protocol/handshake/password_method/request.rs`

**Step 1: 定位两个 impl 块**

Run: `grep -n "impl.*StreamOperation" src/protocol/handshake/password_method/request.rs`

**Step 2: 合并 impl 块**

按照相同模式合并。

**Step 3: 验证编译**

Run: `cargo check --lib`

**Step 4: Commit**

```bash
git add src/protocol/handshake/password_method/request.rs
git commit -m "refactor(password_method/request): merge impl blocks into single StreamOperation

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 9: 更新密码认证 Response 实现

**Files:**
- Modify: `src/protocol/handshake/password_method/response.rs`

**Step 1: 定位两个 impl 块**

Run: `grep -n "impl.*StreamOperation" src/protocol/handshake/password_method/response.rs`

**Step 2: 合并 impl 块**

按照相同模式合并。

**Step 3: 验证编译**

Run: `cargo check --lib`

**Step 4: Commit**

```bash
git add src/protocol/handshake/password_method/response.rs
git commit -m "refactor(password_method/response): merge impl blocks into single StreamOperation

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 10: 运行完整测试套件

**Files:**
- None (testing only)

**Step 1: 运行所有库测试**

Run: `cargo test --lib`
Expected: 所有测试通过

**Step 2: 检查代码覆盖率**

确认测试覆盖率没有降低。

**Step 3: 运行 clippy 检查**

Run: `cargo clippy --lib -- -D warnings`
Expected: 没有警告

**Step 4: 格式化代码**

Run: `cargo fmt`

**Step 5: Commit 格式化**

```bash
git add -u
git commit -m "style: run cargo fmt

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 11: 更新版本号和 CHANGELOG

**Files:**
- Modify: `Cargo.toml:3`
- Create: `CHANGELOG.md` (如果不存在)

**Step 1: 更新 Cargo.toml 版本号**

将 `version = "0.8.0"` 改为 `version = "0.9.0"`

**Step 2: 创建或更新 CHANGELOG.md**

添加 0.9.0 版本的变更记录：

```markdown
# Changelog

## [0.9.0] - 2026-01-28

### Changed
- **BREAKING**: Merged `StreamOperation` and `AsyncStreamOperation` into single `StreamOperation` trait
- **BREAKING**: Removed sync `retrieve_from_stream` method
- All types now have single impl block instead of two
- Reduced codebase by ~400-500 lines of duplicate code

### Migration Guide

If you were implementing both traits:

```rust
// Before
impl StreamOperation for MyType {
    fn retrieve_from_stream(...) { }
    fn write_to_buf(...) { }
    fn len() -> usize { }
}

impl AsyncStreamOperation for MyType {
    async fn retrieve_from_async_stream(...) { }
}

// After
impl StreamOperation for MyType {
    async fn retrieve_from_async_stream(...) { }
    fn write_to_buf(...) { }
    fn len() -> usize { }
}
```

If you were using sync methods, switch to async:

```rust
// Before
let obj = Type::retrieve_from_stream(&mut reader)?;

// After
let obj = Type::retrieve_from_async_stream(&mut reader).await?;
```

## [0.8.0] - 2026-01-28
... (previous versions)
```

**Step 3: Commit 版本更新**

```bash
git add Cargo.toml CHANGELOG.md
git commit -m "chore: bump version to 0.9.0

BREAKING CHANGE: Merged StreamOperation and AsyncStreamOperation traits.
See CHANGELOG.md for migration guide.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## 完成检查清单

- [ ] Task 1: 修改 trait 定义
- [ ] Task 2: 更新 Address 实现
- [ ] Task 3: 更新 Request 实现
- [ ] Task 4: 更新 Response 实现
- [ ] Task 5: 更新 UdpHeader 实现
- [ ] Task 6: 更新握手 Request 实现
- [ ] Task 7: 更新握手 Response 实现
- [ ] Task 8: 更新密码认证 Request 实现
- [ ] Task 9: 更新密码认证 Response 实现
- [ ] Task 10: 运行完整测试套件
- [ ] Task 11: 更新版本号和 CHANGELOG

## 验证标准

- [ ] `cargo test --lib` 通过所有测试
- [ ] `cargo clippy --lib -- -D warnings` 无警告
- [ ] `cargo fmt --check` 代码格式正确
- [ ] 代码行数减少约 400-500 行
- [ ] 每个类型只有一个 impl 块
- [ ] 版本号更新为 0.9.0
- [ ] CHANGELOG.md 包含迁移指南
