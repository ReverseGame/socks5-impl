# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.9.0] - 2026-01-28

### Changed
- **BREAKING**: Merged `StreamOperation` and `AsyncStreamOperation` into single `StreamOperation` trait
- **BREAKING**: Removed sync `retrieve_from_stream` method - all I/O is now async
- **BREAKING**: Removed `TryFrom<Vec<u8>>` and `TryFrom<&[u8]>` implementations for `Address`
- All types now have single impl block instead of two
- Reduced codebase by ~400 lines of duplicate code

### Migration Guide

If you were implementing both traits:

```rust
// Before (0.8.0)
impl StreamOperation for MyType {
    fn retrieve_from_stream<R: Read>(r: &mut R) -> Result<Self> { ... }
    fn write_to_buf<B: BufMut>(&self, buf: &mut B) { ... }
    fn len(&self) -> usize { ... }
}

#[async_trait]
impl AsyncStreamOperation for MyType {
    async fn retrieve_from_async_stream<R: AsyncRead>(r: &mut R) -> Result<Self> { ... }
}

// After (0.9.0)
#[async_trait]
impl StreamOperation for MyType {
    async fn retrieve_from_async_stream<R: AsyncRead>(r: &mut R) -> Result<Self> { ... }
    fn write_to_buf<B: BufMut>(&self, buf: &mut B) { ... }
    fn len(&self) -> usize { ... }
}
```

If you were using sync methods, switch to async:

```rust
// Before (0.8.0)
let addr = Address::retrieve_from_stream(&mut reader)?;

// After (0.9.0)
let addr = Address::retrieve_from_async_stream(&mut reader).await?;
```

If you were using `TryFrom` for `Address`:

```rust
// Before (0.8.0)
let addr = Address::try_from(bytes)?;

// After (0.9.0)
use std::io::Cursor;
let addr = Address::retrieve_from_async_stream(&mut Cursor::new(&bytes)).await?;
```

## [0.8.0] - 2026-01-28

### Changed
- Made tokio a required dependency (no longer optional)
- Removed all `#[cfg(feature = "tokio")]` feature gates
- Simplified feature flags configuration

### Added
- Zero-copy optimizations for UserKey methods (`username()`, `password()`)
- Zero-copy IPv6 address parsing

### Deprecated
- `UserKey::username_arr()` - use `username()` for zero-copy access
- `UserKey::password_arr()` - use `password()` for zero-copy access
