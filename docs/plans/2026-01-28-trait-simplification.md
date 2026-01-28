# StreamOperation 和 AsyncStreamOperation Trait 简化设计

## 目标

简化 SOCKS5 协议实现中的 trait 设计，消除 `StreamOperation` 和 `AsyncStreamOperation` 之间的代码重复。

## 问题分析

当前设计存在以下问题：

1. **代码重复严重**：每个类型需要实现两个几乎相同的 `retrieve` 方法
   - `StreamOperation::retrieve_from_stream` (同步版本)
   - `AsyncStreamOperation::retrieve_from_async_stream` (异步版本)

2. **维护成本高**：8 个实现文件都需要维护两套解析逻辑

3. **语义不清晰**：两个 trait 的职责边界模糊

## 设计方案

### 核心理念

- **只保留一个 trait**：合并原有的两个 trait 为统一的 `StreamOperation`
- **异步优先**：所有 I/O 操作都是异步的
- **职责统一**：序列化 + 异步 I/O 都在一个 trait 中

### Trait 定义

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

### 关键变化

1. **删除旧 StreamOperation trait**
   - 完全移除原来的同步 trait

2. **重命名 AsyncStreamOperation**
   - `AsyncStreamOperation` → `StreamOperation`
   - 保留所有异步方法

3. **合并方法**
   - 从旧 `StreamOperation` 迁移 `write_to_buf` 和 `len`
   - 保留 `retrieve_from_async_stream` 和 `write_to_async_stream`

4. **简化实现**
   - 每个类型只需实现一个 trait
   - 必须实现：`retrieve_from_async_stream`、`write_to_buf`、`len`
   - 自动获得：`is_empty`、`write_to_async_stream`

## 代码迁移

### 影响的文件

需要修改的文件（共 9 个）：

1. `src/protocol/mod.rs` - trait 定义
2. `src/protocol/address.rs` - Address 实现
3. `src/protocol/request.rs` - Request 实现
4. `src/protocol/response.rs` - Response 实现
5. `src/protocol/udp.rs` - UdpHeader 实现
6. `src/protocol/handshake/request.rs` - 握手请求
7. `src/protocol/handshake/response.rs` - 握手响应
8. `src/protocol/handshake/password_method/request.rs` - 密码请求
9. `src/protocol/handshake/password_method/response.rs` - 密码响应

### 迁移步骤

对于每个实现文件：

1. **合并两个 impl 块为一个**
   ```rust
   // 之前：两个 impl 块
   impl StreamOperation for Address {
       fn retrieve_from_stream<R: std::io::Read>(r: &mut R) -> Result<Self> {
           // ... 删除
       }
       fn write_to_buf<B: BufMut>(&self, buf: &mut B) { ... }
       fn len(&self) -> usize { ... }
   }

   #[async_trait::async_trait]
   impl AsyncStreamOperation for Address {
       async fn retrieve_from_async_stream<R>(r: &mut R) -> Result<Self> { ... }
   }

   // 之后：一个 impl 块
   #[async_trait::async_trait]
   impl StreamOperation for Address {
       async fn retrieve_from_async_stream<R>(r: &mut R) -> Result<Self>
       where
           R: AsyncRead + Unpin + Send + ?Sized,
       {
           // 保持不变
       }

       fn write_to_buf<B: BufMut>(&self, buf: &mut B) { ... }
       fn len(&self) -> usize { ... }
   }
   ```

2. **具体操作**
   - 删除整个旧 `impl StreamOperation` 块
   - 将 `impl AsyncStreamOperation` 改为 `impl StreamOperation`
   - 将旧 `impl StreamOperation` 中的 `write_to_buf` 和 `len` 方法移到新的 impl 块中

## 测试修改

### 测试迁移

所有测试保持异步不变，只需更新方法调用：

```rust
// 之前：使用 retrieve_from_stream（已在之前的优化中改为异步）
#[tokio::test]
async fn test_address_async() {
    let addr = Address::from((Ipv4Addr::new(127, 0, 0, 1), 8080));
    let mut buf = Vec::new();
    addr.write_to_async_stream(&mut buf).await.unwrap();
    let addr2 = Address::retrieve_from_async_stream(&mut Cursor::new(&buf))
        .await
        .unwrap();
    assert_eq!(addr, addr2);
}

// 之后：保持不变（因为当前测试已经是异步的）
#[tokio::test]
async fn test_address_async() {
    let addr = Address::from((Ipv4Addr::new(127, 0, 0, 1), 8080));
    let mut buf = Vec::new();
    addr.write_to_async_stream(&mut buf).await.unwrap();
    let addr2 = Address::retrieve_from_async_stream(&mut Cursor::new(&buf))
        .await
        .unwrap();
    assert_eq!(addr, addr2);
}
```

### 验证清单

- [ ] 修改 `src/protocol/mod.rs` trait 定义（删除旧 StreamOperation，重命名 AsyncStreamOperation）
- [ ] 合并 8 个实现文件的 impl 块
- [ ] 运行 `cargo test --lib` 验证所有测试通过
- [ ] 检查是否有其他代码引用了旧的 trait 名称
- [ ] 确保测试覆盖率不降低

## 向后兼容性

**破坏性变更**：

- 删除了 `StreamOperation` trait（旧的同步版本）
- 删除了 `AsyncStreamOperation` trait 名称
- 新增统一的 `StreamOperation` trait（原 AsyncStreamOperation）

**API 变化**：

```rust
// 旧 API
use socks5_impl::protocol::{StreamOperation, AsyncStreamOperation};

impl StreamOperation for Type {
    fn retrieve_from_stream(...) { } // 已删除
    fn write_to_buf(...) { }
    fn len() -> usize { }
}

impl AsyncStreamOperation for Type {
    async fn retrieve_from_async_stream(...) { }
}

// 新 API
use socks5_impl::protocol::StreamOperation;

impl StreamOperation for Type {
    async fn retrieve_from_async_stream(...) { }
    fn write_to_buf(...) { }
    fn len() -> usize { }
}
```

**版本更新**：

- 版本号升级到 `0.9.0`
- 更新 CHANGELOG 说明迁移路径

**迁移指南**（供下游用户）：

1. 删除 `AsyncStreamOperation` 导入
2. 所有 `impl AsyncStreamOperation` 改为 `impl StreamOperation`
3. 如果使用了 `retrieve_from_stream`（同步方法），改为 `retrieve_from_async_stream().await`

## 优势

1. **极致简化**：从两个 trait 合并为一个 trait
2. **消除重复**：每个类型只需一个 impl 块，维护一套逻辑
3. **API 统一**：不再区分同步/异步 trait，只有 `StreamOperation`
4. **减少认知负担**：实现者不需要理解两个 trait 的关系
5. **代码行数减少**：预计减少约 400-500 行重复代码
6. **语义正确**：SOCKS5 协议天然是异步的，异步优先更合理

## 实施计划

1. 修改 `src/protocol/mod.rs` trait 定义
2. 批量修改 8 个实现文件（删除同步 retrieve）
3. 修改所有测试用例（改为异步测试）
4. 运行测试验证
5. 更新版本号和 CHANGELOG
6. 提交 commit
