# Stream Extraction Design

**Date:** 2026-01-28
**Status:** Approved

## Overview

Extract the `Stream` wrapper from `socks5-impl/src/server/connection/stream.rs` into a standalone `stream` crate for reuse across protocol implementations.

## Motivation

The `Stream` type is a generic TCP stream wrapper with async graceful shutdown that's useful for any TCP-based protocol implementation. Extracting it allows:

- Code reuse between `socks5-impl` and `http-impl`
- Independent versioning and testing
- Clean separation of concerns
- Potential future publication to crates.io

## Design Decisions

### 1. Crate Structure

**Name:** `stream` (not `stream-wrapper` or `tcp-utils`)
**Location:** `proxy-protocol-impl/stream/`
**Module structure:** Flat - all code in `src/lib.rs`

```
proxy-protocol-impl/
├── Cargo.toml (workspace)
├── stream/
│   ├── Cargo.toml
│   ├── src/
│   │   └── lib.rs
│   └── README.md
├── socks5-impl/
└── http-impl/
```

### 2. Dependencies

**Minimal, required only:**
```toml
[dependencies]
tokio = { version = "1.48.0", features = ["net", "io-util", "rt"] }
```

**No feature flags.** The crate is tokio-specific since:
- `Stream` wraps `tokio::net::TcpStream`
- Async drop uses `tokio::spawn`
- Making it runtime-agnostic adds complexity without clear benefit

### 3. Public API

**Single public type:** `pub struct Stream`

**Public methods:**
- `new(stream: TcpStream) -> Self`
- `async shutdown(&mut self) -> io::Result<()>`
- `into_inner(self) -> TcpStream`
- TCP option getters/setters: `local_addr()`, `peer_addr()`, `nodelay()`, `set_nodelay()`, `ttl()`, `set_ttl()`, `linger()`

**Trait implementations:**
- `AsyncRead` / `AsyncWrite` (delegate to inner `TcpStream`)
- `Deref` / `DerefMut` (transparent access to `TcpStream` methods)
- `Debug`
- `Drop` (async graceful shutdown with `#[cfg(not(test))]`)

**Private internals:**
- `stream: Option<TcpStream>` field (was `pub(crate)`, now private)
- `get_stream()` / `get_stream_mut()` helper methods

### 4. Code Changes

**From stream.rs:**
1. Change field visibility: `pub(crate) stream` → `stream`
2. Remove doc links to `crate::server::connection::Bind` (doesn't exist in new crate)
3. Add crate-level docs: `#![doc = include_str!("../README.md")]`

**Everything else stays identical** - no behavioral changes.

### 5. Workspace Integration

**Root Cargo.toml:**
```toml
[workspace]
members = [
    "stream",
    "socks5-impl",
    "http-impl"
]
```

**socks5-impl/Cargo.toml:**
```toml
[dependencies]
stream = { path = "../stream" }
```

**Import changes in socks5-impl:**
```rust
// Old:
mod stream;
use stream::Stream;

// New:
use stream::Stream;
```

**Delete:** `socks5-impl/src/server/connection/stream.rs`

### 6. Metadata

**stream/Cargo.toml:**
```toml
[package]
name = "stream"
version = "0.1.0"
authors = ["ssrlive <ssrlivebox@gmail.com>"]
description = "A TcpStream wrapper with async graceful shutdown on drop"
categories = ["network-programming", "asynchronous"]
keywords = ["tcp", "stream", "async", "tokio", "graceful-shutdown"]
edition = "2024"
license = "GPL-3.0-or-later"
```

## Implementation Steps

1. Create `stream/` directory structure
2. Write `stream/Cargo.toml`
3. Create `stream/src/lib.rs` with modified code
4. Write `stream/README.md`
5. Update root `Cargo.toml` workspace members
6. Update `socks5-impl/Cargo.toml` dependencies
7. Update imports in `socks5-impl/src/server/connection/mod.rs`
8. Delete `socks5-impl/src/server/connection/stream.rs`
9. Run `cargo check --workspace` to verify
10. Run `cargo test --workspace` to ensure tests pass

## Testing

- Existing `socks5-impl` tests should continue to pass
- The `#[cfg(not(test))]` on `Drop` impl remains unchanged
- No new tests needed - this is a refactor, not a feature change

## Future Possibilities

- Publish to crates.io for broader reuse
- Add configuration options (timeout values, buffer sizes)
- Support for other async runtimes (if demand exists)
- Additional stream wrappers (rate limiting, metrics, etc.)

## Non-Goals

- Changing `Stream` behavior or API
- Supporting multiple async runtimes
- Adding features beyond what currently exists
- Breaking changes to `socks5-impl`
