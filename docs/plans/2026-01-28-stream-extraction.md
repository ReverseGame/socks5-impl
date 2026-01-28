# Stream Extraction Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Extract the `Stream` wrapper into a standalone `stream` crate for reuse across protocol implementations.

**Architecture:** Move `socks5-impl/src/server/connection/stream.rs` into a new workspace member `stream/`, update all imports, and make `socks5-impl` depend on the new crate.

**Tech Stack:** Rust 2024 edition, tokio 1.48.0, workspace dependencies

---

## Task 1: Create stream crate structure

**Files:**
- Create: `stream/Cargo.toml`
- Create: `stream/src/lib.rs`
- Create: `stream/README.md`

**Step 1: Create stream crate directory**

```bash
mkdir -p stream/src
```

Run: `ls -la stream/`
Expected: Directory created with `src` subdirectory

**Step 2: Write stream/Cargo.toml**

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

[dependencies]
tokio = { version = "1.48.0", features = ["net", "io-util", "rt"] }
```

Run: `cat stream/Cargo.toml`
Expected: File contents match above

**Step 3: Write stream/README.md**

```markdown
# stream

A `TcpStream` wrapper with async graceful shutdown on drop.

## Features

- Transparent wrapper around `tokio::net::TcpStream`
- Automatic graceful shutdown on drop (using `tokio::spawn`)
- Implements `AsyncRead`, `AsyncWrite`, `Deref`, and `DerefMut`
- Full access to TCP socket options

## Usage

```rust
use stream::Stream;
use tokio::net::TcpStream;

let tcp_stream = TcpStream::connect("127.0.0.1:8080").await?;
let stream = Stream::new(tcp_stream);

// Use as TcpStream via Deref
stream.set_nodelay(true)?;

// Explicit shutdown (recommended for critical connections)
stream.shutdown().await?;

// Or let it drop and shutdown happens in background
```

## Design

The `Stream` type wraps `Option<TcpStream>` internally to enable async drop:

1. On drop, the inner `TcpStream` is extracted
2. A background task is spawned with `tokio::spawn`
3. Graceful shutdown happens asynchronously

For guaranteed shutdown completion, call `shutdown()` explicitly before dropping.

## License

GPL-3.0-or-later
```

Run: `wc -l stream/README.md`
Expected: README created

**Step 4: Commit structure**

```bash
git add stream/
git commit -m "chore: create stream crate structure"
```

Run: `git log -1 --oneline`
Expected: Commit created with message "chore: create stream crate structure"

---

## Task 2: Migrate Stream implementation

**Files:**
- Create: `stream/src/lib.rs`
- Read: `socks5-impl/src/server/connection/stream.rs`

**Step 1: Copy and modify Stream code to stream/src/lib.rs**

Create `stream/src/lib.rs` with the following content (copied from `socks5-impl/src/server/connection/stream.rs` with modifications):

```rust
#![doc = include_str!("../README.md")]

use std::net::SocketAddr;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::TcpStream;

/// A wrapper around `TcpStream` that performs async graceful shutdown on drop.
///
/// When this struct is dropped, it will spawn a background task to perform
/// a graceful TCP shutdown. This ensures proper connection termination without
/// blocking the current task.
///
/// # Performance Note
///
/// The async drop mechanism uses `tokio::spawn` to avoid blocking. This means:
/// - Drop is non-blocking and won't impact async task performance
/// - Graceful shutdown happens in the background
/// - If you need to ensure shutdown completes, call `shutdown()` explicitly before dropping
#[derive(Debug)]
pub struct Stream {
    stream: Option<TcpStream>,
}

impl Stream {
    /// Get internal TcpStream reference
    #[inline]
    fn get_stream(&self) -> &TcpStream {
        self.stream.as_ref().expect("Stream has been consumed")
    }

    /// Get internal TcpStream mutable reference
    #[inline]
    fn get_stream_mut(&mut self) -> &mut TcpStream {
        self.stream.as_mut().expect("Stream has been consumed")
    }
}

impl Stream {
    #[inline]
    pub fn new(stream: TcpStream) -> Self {
        Self { stream: Some(stream) }
    }

    /// Causes the other peer to receive a read of length 0, indicating that no more data will be sent.
    /// This only closes the stream in one direction (graceful shutdown).
    ///
    /// # Note
    ///
    /// While `Stream` performs async shutdown on drop, calling this method explicitly ensures
    /// that the shutdown completes and any errors are reported. This is recommended for
    /// critical connections where you need to ensure proper closure.
    #[inline]
    pub async fn shutdown(&mut self) -> std::io::Result<()> {
        self.get_stream_mut().shutdown().await
    }

    /// Consumes the `Stream` and returns the inner `TcpStream`.
    ///
    /// This method extracts the underlying `TcpStream` without triggering the async drop
    /// behavior, giving you full control over the connection lifecycle.
    #[inline]
    pub fn into_inner(mut self) -> TcpStream {
        self.stream.take().expect("Stream has been consumed")
    }

    /// Returns the local address that this stream is bound to.
    #[inline]
    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.get_stream().local_addr()
    }

    /// Returns the remote address that this stream is connected to.
    #[inline]
    pub fn peer_addr(&self) -> std::io::Result<SocketAddr> {
        self.get_stream().peer_addr()
    }

    /// Reads the linger duration for this socket by getting the `SO_LINGER` option.
    #[inline]
    pub fn linger(&self) -> std::io::Result<Option<Duration>> {
        self.get_stream().linger()
    }

    /// Gets the value of the `TCP_NODELAY` option on this socket.
    #[inline]
    pub fn nodelay(&self) -> std::io::Result<bool> {
        self.get_stream().nodelay()
    }

    /// Sets the value of the `TCP_NODELAY` option on this socket.
    ///
    /// If set, this option disables the Nagle algorithm. This means that segments are always sent as soon as possible,
    /// even if there is only a small amount of data. When not set, data is buffered until there is a sufficient amount to send out,
    /// thereby avoiding the frequent sending of small packets.
    pub fn set_nodelay(&self, nodelay: bool) -> std::io::Result<()> {
        self.get_stream().set_nodelay(nodelay)
    }

    /// Gets the value of the `IP_TTL` option for this socket.
    pub fn ttl(&self) -> std::io::Result<u32> {
        self.get_stream().ttl()
    }

    /// Sets the value for the `IP_TTL` option on this socket.
    ///
    /// This value sets the time-to-live field that is used in every packet sent from this socket.
    pub fn set_ttl(&self, ttl: u32) -> std::io::Result<()> {
        self.get_stream().set_ttl(ttl)
    }
}

impl Deref for Stream {
    type Target = TcpStream;

    fn deref(&self) -> &Self::Target {
        self.get_stream()
    }
}

impl DerefMut for Stream {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_stream_mut()
    }
}

// Implement AsyncRead trait by delegating to inner TcpStream
impl AsyncRead for Stream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(self.get_stream_mut()).poll_read(cx, buf)
    }
}

// Implement AsyncWrite trait by delegating to inner TcpStream
impl AsyncWrite for Stream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(self.get_stream_mut()).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(self.get_stream_mut()).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(self.get_stream_mut()).poll_shutdown(cx)
    }

    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(self.get_stream_mut()).poll_write_vectored(cx, bufs)
    }

    fn is_write_vectored(&self) -> bool {
        self.get_stream().is_write_vectored()
    }
}

// Implement async Drop by spawning background task for graceful shutdown
//
// This implementation uses tokio::spawn to async execute shutdown in the background, avoiding blocking the current task.
// Compared to the previous block_in_place version, this approach:
// 1. Does not block async task execution
// 2. Does not occupy thread pool resources
// 3. Better performance in high concurrency scenarios
//
// Note: Since shutdown executes in the background, if you need to ensure shutdown completes,
// it's recommended to explicitly call shutdown() method before drop.
#[cfg(not(test))]
impl Drop for Stream {
    fn drop(&mut self) {
        // Take TcpStream from Option
        if let Some(stream) = self.stream.take() {
            // Try to async execute shutdown in current tokio runtime
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.spawn(async move {
                    let mut stream = stream;
                    // Ignore errors since connection may already be closed
                    let _ = stream.shutdown().await;
                });
            }
            // If not in tokio runtime, stream will be directly dropped,
            // TCP protocol stack will handle connection closure
        }
    }
}
```

**Key changes from original:**
1. Changed `pub(crate) stream` to `stream` (private field)
2. Removed doc link references to `crate::server::connection::Bind`
3. Added crate-level docs: `#![doc = include_str!("../README.md")]`

Run: `wc -l stream/src/lib.rs`
Expected: File created with ~205 lines

**Step 2: Verify stream crate builds**

```bash
cargo check -p stream
```

Run: `cargo check -p stream`
Expected: SUCCESS - "Checking stream v0.1.0" with no errors

**Step 3: Commit stream implementation**

```bash
git add stream/src/lib.rs
git commit -m "feat(stream): implement TcpStream wrapper with async drop"
```

Run: `git log -1 --oneline`
Expected: Commit with message "feat(stream): implement TcpStream wrapper with async drop"

---

## Task 3: Update workspace configuration

**Files:**
- Modify: `Cargo.toml:1-6`

**Step 1: Update root Cargo.toml workspace members**

Edit `Cargo.toml` to add `stream` to workspace members:

```toml
[workspace]
members = [
    "stream",
    "socks5-impl",
    "http-impl"
]
```

Run: `cat Cargo.toml`
Expected: File shows stream as first member

**Step 2: Verify workspace structure**

```bash
cargo metadata --format-version 1 | grep -o '"name":"stream"' | head -1
```

Run: Above command
Expected: Output shows `"name":"stream"`

**Step 3: Commit workspace update**

```bash
git add Cargo.toml
git commit -m "chore: add stream crate to workspace"
```

Run: `git log -1 --oneline`
Expected: Commit created

---

## Task 4: Add stream dependency to socks5-impl

**Files:**
- Modify: `socks5-impl/Cargo.toml:23-35`

**Step 1: Add stream dependency**

In `socks5-impl/Cargo.toml`, add `stream` to dependencies section:

```toml
[dependencies]
async-trait = { version = "0.1.89", optional = true }
bytes = "1.11.0"
percent-encoding = "2.3.2"
serde = { version = "1.0.228", features = ["derive"], optional = true }
stream = { path = "../stream" }
thiserror = "2.0.17"
tokio = { version = "1.48.0", default-features = false, features = [
    "net",
    "io-util",
    "time",
    "macros",
    "rt",
], optional = true }
```

Run: `grep 'stream = ' socks5-impl/Cargo.toml`
Expected: Shows `stream = { path = "../stream" }`

**Step 2: Verify dependency resolution**

```bash
cargo check -p socks5-impl
```

Run: `cargo check -p socks5-impl`
Expected: May fail with import errors (expected at this stage)

**Step 3: Commit dependency addition**

```bash
git add socks5-impl/Cargo.toml
git commit -m "chore(socks5-impl): add stream crate dependency"
```

Run: `git log -1 --oneline`
Expected: Commit created

---

## Task 5: Update imports in socks5-impl

**Files:**
- Modify: `socks5-impl/src/server/connection/mod.rs:1-11`

**Step 1: Update mod.rs imports**

Edit `socks5-impl/src/server/connection/mod.rs`:

```rust
use self::{associate::UdpAssociate, bind::Bind, connect::Connect};
use crate::protocol::{self, Address, AsyncStreamOperation, AuthMethod, Command, handshake};
use stream::Stream;  // Changed from: use crate::server::connection::stream::Stream;
use std::time::Duration;
use tokio::net::TcpStream;
use crate::server::AuthAdaptor;

pub mod associate;
pub mod bind;
pub mod connect;
// Removed: pub mod stream;
```

**Key changes:**
1. Changed `use crate::server::connection::stream::Stream;` to `use stream::Stream;`
2. Removed `pub mod stream;` declaration

Run: `head -12 socks5-impl/src/server/connection/mod.rs`
Expected: Shows updated imports without `pub mod stream;`

**Step 2: Verify all files using Stream**

```bash
grep -r "use.*stream::Stream" socks5-impl/src/server/connection/ --include="*.rs"
```

Run: Above command
Expected: Only mod.rs shows the import (others use it via mod.rs)

**Step 3: Commit import updates**

```bash
git add socks5-impl/src/server/connection/mod.rs
git commit -m "refactor(socks5-impl): update Stream imports to use stream crate"
```

Run: `git log -1 --oneline`
Expected: Commit created

---

## Task 6: Remove old stream.rs file

**Files:**
- Delete: `socks5-impl/src/server/connection/stream.rs`

**Step 1: Delete stream.rs**

```bash
git rm socks5-impl/src/server/connection/stream.rs
```

Run: `git status`
Expected: Shows `deleted: socks5-impl/src/server/connection/stream.rs`

**Step 2: Verify no dangling references**

```bash
grep -r "mod stream" socks5-impl/src/ --include="*.rs"
```

Run: Above command
Expected: No output (all references removed)

**Step 3: Commit deletion**

```bash
git commit -m "refactor(socks5-impl): remove old stream.rs, now using stream crate"
```

Run: `git log -1 --oneline`
Expected: Commit with message about removing stream.rs

---

## Task 7: Verify workspace builds

**Files:**
- None (verification only)

**Step 1: Clean build to ensure no stale artifacts**

```bash
cargo clean
```

Run: `cargo clean`
Expected: Removes target directory

**Step 2: Build entire workspace**

```bash
cargo build --workspace
```

Run: `cargo build --workspace`
Expected: SUCCESS - All crates build without errors

**Step 3: Check all workspace members**

```bash
cargo check --workspace --all-features
```

Run: `cargo check --workspace --all-features`
Expected: SUCCESS - No warnings or errors

**Step 4: Commit verification note**

No commit needed - verification step only.

---

## Task 8: Run tests

**Files:**
- None (testing only)

**Step 1: Run all workspace tests**

```bash
cargo test --workspace
```

Run: `cargo test --workspace`
Expected: All tests pass (existing socks5-impl tests should work unchanged)

**Step 2: Run socks5-impl specific tests**

```bash
cargo test -p socks5-impl --lib
```

Run: `cargo test -p socks5-impl --lib`
Expected: All library tests pass

**Step 3: Verify examples still compile**

```bash
cargo check --examples -p socks5-impl
```

Run: `cargo check --examples -p socks5-impl`
Expected: All examples compile successfully

**Step 4: No commit needed**

Verification step only.

---

## Task 9: Final validation

**Files:**
- None (validation only)

**Step 1: Verify workspace structure**

```bash
ls -la stream/ socks5-impl/ http-impl/ | grep -E "^d|Cargo.toml"
```

Run: Above command
Expected: Shows directory structure with all Cargo.toml files

**Step 2: Verify Stream is exported from stream crate**

```bash
cargo doc -p stream --no-deps --open
```

Run: `cargo doc -p stream --no-deps`
Expected: Docs build successfully showing Stream type

**Step 3: Verify socks5-impl uses stream crate**

```bash
cargo tree -p socks5-impl | grep stream
```

Run: Above command
Expected: Shows `stream v0.1.0` as dependency

**Step 4: Create final summary commit**

```bash
git add -A
git commit -m "refactor: complete stream extraction to standalone crate

Extract Stream wrapper from socks5-impl into reusable stream crate.
This enables code sharing between socks5-impl and http-impl.

Changes:
- Created stream/ crate with TcpStream wrapper
- Updated socks5-impl to depend on stream crate
- Removed socks5-impl/src/server/connection/stream.rs
- All tests passing, no behavioral changes"
```

Run: `git log -1 --stat`
Expected: Shows comprehensive commit with all changes

---

## Task 10: Documentation and verification

**Files:**
- None (documentation validation)

**Step 1: Verify README exists and is correct**

```bash
cat stream/README.md | head -20
```

Run: Above command
Expected: Shows README with usage examples

**Step 2: Verify crate-level docs render correctly**

```bash
cargo doc --workspace --no-deps
```

Run: `cargo doc --workspace --no-deps`
Expected: Docs build for all crates

**Step 3: Run final workspace check**

```bash
cargo check --workspace --all-targets --all-features
```

Run: `cargo check --workspace --all-targets --all-features`
Expected: SUCCESS with no warnings

---

## Completion Checklist

- [ ] stream crate created with correct structure
- [ ] stream/Cargo.toml has correct metadata
- [ ] stream/src/lib.rs contains Stream implementation
- [ ] stream/README.md exists with usage docs
- [ ] Root Cargo.toml updated with stream member
- [ ] socks5-impl/Cargo.toml has stream dependency
- [ ] socks5-impl imports updated to use stream crate
- [ ] Old stream.rs file removed
- [ ] Workspace builds successfully
- [ ] All tests pass
- [ ] Documentation builds
- [ ] No behavioral changes to existing code

## Notes

- This is a pure refactor - no functional changes to Stream
- The `#[cfg(not(test))]` on Drop remains unchanged
- All existing socks5-impl tests should pass without modification
- http-impl can now use stream by adding `stream = { path = "../stream" }` to its Cargo.toml
