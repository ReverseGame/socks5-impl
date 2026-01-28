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

- `StreamOperation` 作为基础 trait，只包含序列化相关方法（无 I/O）
- `AsyncStreamOperation` 作为扩展 trait，添加异步 I/O 方法
- 异步实现是一等公民，移除同步 I/O 方法

### Trait 定义

```rust
/// 基础序列化 trait（无 I/O 操作）
pub trait StreamOperation {
    /// 将对象序列化到缓冲区
    fn write_to_buf<B: BufMut>(&self, buf: &mut B);

    /// 返回序列化后的字节长度
    fn len(&self) -> usize;

    /// 判断是否为空
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// 异步 I/O 扩展 trait
#[async_trait::async_trait]
pub trait AsyncStreamOperation: StreamOperation {
    /// 从异步流中读取并反序列化对象
    async fn retrieve_from_async_stream<R>(r: &mut R) -> std::io::Result<Self>
    where
        R: AsyncRead + Unpin + Send + ?Sized,
        Self: Sized;

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

1. **移除同步 I/O**
   - 删除 `StreamOperation::retrieve_from_stream`
   - 删除 `StreamOperation::write_to_stream`

2. **简化实现**
   - 每个类型只需实现一个 `retrieve` 方法（异步版本）
   - 保留 `write_to_buf` 和 `len` 方法

3. **Trait 约束**
   - `AsyncStreamOperation: StreamOperation` 显式约束
   - 实现者必须同时提供序列化和异步 I/O

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

1. **删除同步 retrieve 实现**
   ```rust
   // 删除这部分代码
   impl StreamOperation for Address {
       fn retrieve_from_stream<R: std::io::Read>(r: &mut R) -> Result<Self> {
           // ... 50+ 行代码
       }
       // 保留这些
       fn write_to_buf<B: BufMut>(&self, buf: &mut B) { ... }
       fn len(&self) -> usize { ... }
   }
   ```

2. **保留异步实现**
   ```rust
   #[async_trait::async_trait]
   impl AsyncStreamOperation for Address {
       async fn retrieve_from_async_stream<R>(r: &mut R) -> Result<Self>
       where
           R: AsyncRead + Unpin + Send + ?Sized,
       {
           // 保持不变
       }
   }
   ```

## 测试修改

### 测试迁移

所有使用 `retrieve_from_stream` 的测试需要改为异步：

```rust
// 之前
#[test]
fn test_address() {
    let addr = Address::from((Ipv4Addr::new(127, 0, 0, 1), 8080));
    let mut buf = Vec::new();
    addr.write_to_buf(&mut buf);
    let addr2 = Address::retrieve_from_stream(&mut Cursor::new(&buf)).unwrap();
    assert_eq!(addr, addr2);
}

// 之后
#[tokio::test]
async fn test_address() {
    let addr = Address::from((Ipv4Addr::new(127, 0, 0, 1), 8080));
    let mut buf = Vec::new();
    addr.write_to_buf(&mut buf);
    let addr2 = Address::retrieve_from_async_stream(&mut Cursor::new(&buf))
        .await
        .unwrap();
    assert_eq!(addr, addr2);
}
```

### 验证清单

- [ ] 所有 `#[test]` 改为 `#[tokio::test]`
- [ ] 所有 `retrieve_from_stream` 改为 `retrieve_from_async_stream().await`
- [ ] 运行 `cargo test --lib` 验证所有测试通过
- [ ] 确保测试覆盖率不降低

## 向后兼容性

**破坏性变更**：

- 移除了公开 API `StreamOperation::retrieve_from_stream`
- 移除了公开 API `StreamOperation::write_to_stream`

**版本更新**：

- 版本号升级到 `0.9.0`
- 更新 CHANGELOG 说明迁移路径

**迁移指南**（供下游用户）：

```rust
// 旧代码
let obj = Type::retrieve_from_stream(&mut reader)?;

// 新代码
let obj = Type::retrieve_from_async_stream(&mut async_reader).await?;
```

## 优势

1. **消除重复**：每个类型只需维护一套解析逻辑
2. **职责清晰**：
   - `StreamOperation` = 序列化（无副作用）
   - `AsyncStreamOperation` = I/O + 序列化
3. **简化维护**：减少约 400 行重复代码
4. **语义正确**：SOCKS5 协议天然是异步的，异步优先更合理

## 实施计划

1. 修改 `src/protocol/mod.rs` trait 定义
2. 批量修改 8 个实现文件（删除同步 retrieve）
3. 修改所有测试用例（改为异步测试）
4. 运行测试验证
5. 更新版本号和 CHANGELOG
6. 提交 commit
