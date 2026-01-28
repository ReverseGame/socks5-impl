# HTTP Protocol Implementation (http-impl) Extraction Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Extract HTTP protocol parsing and building logic into a standalone `http-impl` crate with minimal responsibility (protocol only, no business logic)

**Architecture:** Type-state pattern for compile-time safety, zero-copy with Bytes for performance, optional async support via feature flag

**Tech Stack:**
- http crate (Uri, Method, HeaderMap)
- bytes (zero-copy buffers)
- httparse (HTTP/1.1 parsing)
- thiserror (error handling)
- base64 (Basic Auth)
- tokio (optional async, feature-gated)

---

## Design Overview

### 1. Architecture: Type-State Pattern

```rust
// 构建器：编译期类型安全
pub struct HttpRequestBuilder<S> {
    method: Option<Method>,
    uri: Option<Uri>,
    headers: HeaderMap,
    body: Bytes,
    _state: PhantomData<S>,
}

// 状态标记
pub struct Building;
pub struct Complete;

impl HttpRequestBuilder<Building> {
    pub fn method(self, method: Method) -> Self { /* ... */ }
    pub fn uri(self, uri: Uri) -> Self { /* ... */ }
    pub fn build(self) -> HttpRequestBuilder<Complete> { /* ... */ }
}

impl HttpRequestBuilder<Complete> {
    pub fn finish(self) -> HttpRequest { /* ... */ }
}
```

### 2. Core Types

```rust
pub struct HttpRequest {
    method: Method,           // from http crate
    uri: Uri,                 // from http crate
    headers: HeaderMap,       // from http crate
    body: Bytes,              // zero-copy body
    raw_bytes: Bytes,         // preserved for efficient forwarding
}

pub struct HttpResponse {
    status: StatusCode,
    headers: HeaderMap,
    body: Bytes,
    raw_bytes: Bytes,
}
```

### 3. Error Handling

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HttpError {
    #[error("Invalid HTTP request: {0}")]
    InvalidRequest(String),

    #[error("Invalid HTTP response: {0}")]
    InvalidResponse(String),

    #[error("Invalid URI: {0}")]
    InvalidUri(String),

    #[error("Invalid header: {0}")]
    InvalidHeader(String),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[cfg(feature = "async")]
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, HttpError>;
```

### 4. Async Extension (Feature-Gated)

```rust
#[cfg(feature = "async")]
pub mod async_ext {
    use tokio::io::{AsyncRead, AsyncReadExt};

    impl HttpRequest {
        pub async fn from_stream<S>(stream: &mut S) -> Result<Self>
        where
            S: AsyncRead + Unpin,
        {
            // Read and parse HTTP request from async stream
        }
    }

    impl HttpResponse {
        pub async fn write_to_stream<S>(&self, stream: &mut S) -> Result<()>
        where
            S: AsyncWrite + Unpin,
        {
            // Write HTTP response to async stream
        }
    }
}
```

### 5. Zero-Copy Design

保留原始字节以支持高效转发：

```rust
impl HttpRequest {
    /// 获取原始请求字节（用于代理转发）
    pub fn raw_bytes(&self) -> &Bytes {
        &self.raw_bytes
    }
}
```

### 6. Authentication Support

```rust
pub struct BasicAuth {
    pub username: String,
    pub password: String,
}

impl HttpRequest {
    /// 从 Proxy-Authorization header 解析 Basic Auth
    pub fn parse_basic_auth(&self) -> Result<Option<BasicAuth>> {
        // Implementation
    }
}
```

---

## Implementation Tasks

### Task 1: Project Structure Setup

**Files:**
- Modify: `http-impl/Cargo.toml`
- Modify: `http-impl/src/lib.rs`
- Create: `http-impl/src/error.rs`
- Create: `http-impl/README.md`

**Step 1: Update Cargo.toml dependencies**

```toml
[package]
name = "http-impl"
version = "0.1.0"
edition = "2021"
license = "GPL-3.0-or-later"

[dependencies]
http = "1.0"
bytes = "1.0"
thiserror = "1.0"
httparse = "1.8"
base64 = "0.21"

[features]
default = []
async = ["tokio"]

[dependencies.tokio]
version = "1.0"
features = ["io-util"]
optional = true

[dev-dependencies]
tokio = { version = "1.0", features = ["full"] }
```

**Step 2: Create error module**

File: `http-impl/src/error.rs`

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HttpError {
    #[error("Invalid HTTP request: {0}")]
    InvalidRequest(String),

    #[error("Invalid HTTP response: {0}")]
    InvalidResponse(String),

    #[error("Invalid URI: {0}")]
    InvalidUri(String),

    #[error("Invalid header: {0}")]
    InvalidHeader(String),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[cfg(feature = "async")]
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, HttpError>;
```

**Step 3: Initialize lib.rs with module declarations**

File: `http-impl/src/lib.rs`

```rust
#![doc = include_str!("../README.md")]

pub mod error;
pub mod request;
pub mod response;
pub mod auth;

#[cfg(feature = "async")]
pub mod async_ext;

pub use error::{HttpError, Result};
pub use request::{HttpRequest, HttpRequestBuilder};
pub use response::{HttpResponse, HttpResponseBuilder};
pub use auth::BasicAuth;
```

**Step 4: Create README.md**

File: `http-impl/README.md`

```markdown
# http-impl

A minimal HTTP/1.1 protocol implementation with zero-copy design.

## Features

- Type-state pattern for compile-time safety
- Zero-copy with `bytes::Bytes`
- Optional async support (feature-gated)
- HTTP Basic Auth support
- Reuses `http` crate types (Uri, Method, HeaderMap)

## Usage

```rust
use http_impl::{HttpRequest, HttpRequestBuilder};
use http::{Method, Uri};

// Build request
let request = HttpRequestBuilder::new()
    .method(Method::GET)
    .uri("/path".parse::<Uri>().unwrap())
    .header("Host", "example.com")
    .build()
    .finish();

// Parse from bytes
let request = HttpRequest::parse(b"GET / HTTP/1.1\r\n\r\n")?;
```

## License

GPL-3.0-or-later
```

**Step 5: Run cargo check**

```bash
cd http-impl && cargo check
```

Expected: Compile errors about missing modules (will implement next)

**Step 6: Commit**

```bash
git add http-impl/Cargo.toml http-impl/src/lib.rs http-impl/src/error.rs http-impl/README.md
git commit -m "feat(http-impl): initialize project structure with error types"
```

---

### Task 2: Implement HttpRequest Core

**Files:**
- Create: `http-impl/src/request.rs`
- Create: `http-impl/tests/request_test.rs`

**Step 1: Write failing test**

File: `http-impl/tests/request_test.rs`

```rust
use http_impl::{HttpRequest, HttpRequestBuilder};
use http::{Method, Uri};

#[test]
fn test_parse_simple_get_request() {
    let raw = b"GET /path HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let request = HttpRequest::parse(raw).unwrap();

    assert_eq!(request.method(), &Method::GET);
    assert_eq!(request.uri().path(), "/path");
    assert_eq!(request.header("Host").unwrap(), "example.com");
}

#[test]
fn test_builder_pattern() {
    let request = HttpRequestBuilder::new()
        .method(Method::POST)
        .uri("/api".parse::<Uri>().unwrap())
        .header("Content-Type", "application/json")
        .body(b"{}".to_vec())
        .build()
        .finish();

    assert_eq!(request.method(), &Method::POST);
    assert_eq!(request.body().as_ref(), b"{}");
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test --test request_test
```

Expected: FAIL with "module not found: http_impl"

**Step 3: Implement HttpRequest**

File: `http-impl/src/request.rs`

```rust
use crate::error::{HttpError, Result};
use bytes::Bytes;
use http::{HeaderMap, HeaderName, HeaderValue, Method, Uri};
use std::marker::PhantomData;

pub struct HttpRequest {
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
    raw_bytes: Bytes,
}

impl HttpRequest {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);

        let status = req.parse(data)
            .map_err(|e| HttpError::InvalidRequest(e.to_string()))?;

        let header_len = match status {
            httparse::Status::Complete(len) => len,
            httparse::Status::Partial => {
                return Err(HttpError::InvalidRequest("Incomplete request".to_string()));
            }
        };

        let method = req.method
            .ok_or_else(|| HttpError::InvalidRequest("Missing method".to_string()))?
            .parse::<Method>()
            .map_err(|e| HttpError::InvalidRequest(e.to_string()))?;

        let uri = req.path
            .ok_or_else(|| HttpError::InvalidRequest("Missing path".to_string()))?
            .parse::<Uri>()
            .map_err(|_| HttpError::InvalidUri("Invalid URI".to_string()))?;

        let mut header_map = HeaderMap::new();
        for header in req.headers {
            let name = HeaderName::from_bytes(header.name.as_bytes())
                .map_err(|e| HttpError::InvalidHeader(e.to_string()))?;
            let value = HeaderValue::from_bytes(header.value)
                .map_err(|e| HttpError::InvalidHeader(e.to_string()))?;
            header_map.insert(name, value);
        }

        let body = Bytes::copy_from_slice(&data[header_len..]);
        let raw_bytes = Bytes::copy_from_slice(data);

        Ok(Self {
            method,
            uri,
            headers: header_map,
            body,
            raw_bytes,
        })
    }

    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn uri(&self) -> &Uri {
        &self.uri
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }

    pub fn body(&self) -> &Bytes {
        &self.body
    }

    pub fn raw_bytes(&self) -> &Bytes {
        &self.raw_bytes
    }
}

// Builder pattern with type-state
pub struct HttpRequestBuilder<S> {
    method: Option<Method>,
    uri: Option<Uri>,
    headers: HeaderMap,
    body: Bytes,
    _state: PhantomData<S>,
}

pub struct Building;
pub struct Complete;

impl HttpRequestBuilder<Building> {
    pub fn new() -> Self {
        Self {
            method: None,
            uri: None,
            headers: HeaderMap::new(),
            body: Bytes::new(),
            _state: PhantomData,
        }
    }

    pub fn method(mut self, method: Method) -> Self {
        self.method = Some(method);
        self
    }

    pub fn uri(mut self, uri: Uri) -> Self {
        self.uri = Some(uri);
        self
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        if let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(name.as_bytes()),
            HeaderValue::from_str(value),
        ) {
            self.headers.insert(name, value);
        }
        self
    }

    pub fn body(mut self, body: Vec<u8>) -> Self {
        self.body = Bytes::from(body);
        self
    }

    pub fn build(self) -> HttpRequestBuilder<Complete> {
        HttpRequestBuilder {
            method: self.method,
            uri: self.uri,
            headers: self.headers,
            body: self.body,
            _state: PhantomData,
        }
    }
}

impl HttpRequestBuilder<Complete> {
    pub fn finish(self) -> HttpRequest {
        let method = self.method.unwrap_or(Method::GET);
        let uri = self.uri.unwrap_or_else(|| "/".parse().unwrap());

        // Build raw bytes for forwarding
        let mut raw = Vec::new();
        raw.extend_from_slice(method.as_str().as_bytes());
        raw.extend_from_slice(b" ");
        raw.extend_from_slice(uri.path().as_bytes());
        raw.extend_from_slice(b" HTTP/1.1\r\n");

        for (name, value) in &self.headers {
            raw.extend_from_slice(name.as_str().as_bytes());
            raw.extend_from_slice(b": ");
            raw.extend_from_slice(value.as_bytes());
            raw.extend_from_slice(b"\r\n");
        }
        raw.extend_from_slice(b"\r\n");
        raw.extend_from_slice(&self.body);

        HttpRequest {
            method,
            uri,
            headers: self.headers,
            body: self.body,
            raw_bytes: Bytes::from(raw),
        }
    }
}

impl Default for HttpRequestBuilder<Building> {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 4: Run tests**

```bash
cargo test --test request_test
```

Expected: PASS

**Step 5: Commit**

```bash
git add http-impl/src/request.rs http-impl/tests/request_test.rs
git commit -m "feat(http-impl): implement HttpRequest with builder pattern"
```

---

### Task 3: Implement HttpResponse

**Files:**
- Create: `http-impl/src/response.rs`
- Create: `http-impl/tests/response_test.rs`

**Step 1: Write failing test**

File: `http-impl/tests/response_test.rs`

```rust
use http_impl::{HttpResponse, HttpResponseBuilder};
use http::StatusCode;

#[test]
fn test_parse_simple_response() {
    let raw = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nHello";
    let response = HttpResponse::parse(raw).unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.header("Content-Type").unwrap(), "text/plain");
    assert_eq!(response.body().as_ref(), b"Hello");
}

#[test]
fn test_builder_pattern() {
    let response = HttpResponseBuilder::new()
        .status(StatusCode::CREATED)
        .header("Content-Type", "application/json")
        .body(b"{}".to_vec())
        .build()
        .finish();

    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(response.body().as_ref(), b"{}");
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test --test response_test
```

Expected: FAIL with "cannot find type `HttpResponse`"

**Step 3: Implement HttpResponse**

File: `http-impl/src/response.rs`

```rust
use crate::error::{HttpError, Result};
use bytes::Bytes;
use http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use std::marker::PhantomData;

pub struct HttpResponse {
    status: StatusCode,
    headers: HeaderMap,
    body: Bytes,
    raw_bytes: Bytes,
}

impl HttpResponse {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut resp = httparse::Response::new(&mut headers);

        let status = resp.parse(data)
            .map_err(|e| HttpError::InvalidResponse(e.to_string()))?;

        let header_len = match status {
            httparse::Status::Complete(len) => len,
            httparse::Status::Partial => {
                return Err(HttpError::InvalidResponse("Incomplete response".to_string()));
            }
        };

        let status_code = resp.code
            .ok_or_else(|| HttpError::InvalidResponse("Missing status code".to_string()))?;

        let status = StatusCode::from_u16(status_code)
            .map_err(|e| HttpError::InvalidResponse(e.to_string()))?;

        let mut header_map = HeaderMap::new();
        for header in resp.headers {
            let name = HeaderName::from_bytes(header.name.as_bytes())
                .map_err(|e| HttpError::InvalidHeader(e.to_string()))?;
            let value = HeaderValue::from_bytes(header.value)
                .map_err(|e| HttpError::InvalidHeader(e.to_string()))?;
            header_map.insert(name, value);
        }

        let body = Bytes::copy_from_slice(&data[header_len..]);
        let raw_bytes = Bytes::copy_from_slice(data);

        Ok(Self {
            status,
            headers: header_map,
            body,
            raw_bytes,
        })
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }

    pub fn body(&self) -> &Bytes {
        &self.body
    }

    pub fn raw_bytes(&self) -> &Bytes {
        &self.raw_bytes
    }
}

// Builder pattern with type-state
pub struct HttpResponseBuilder<S> {
    status: Option<StatusCode>,
    headers: HeaderMap,
    body: Bytes,
    _state: PhantomData<S>,
}

pub struct Building;
pub struct Complete;

impl HttpResponseBuilder<Building> {
    pub fn new() -> Self {
        Self {
            status: None,
            headers: HeaderMap::new(),
            body: Bytes::new(),
            _state: PhantomData,
        }
    }

    pub fn status(mut self, status: StatusCode) -> Self {
        self.status = Some(status);
        self
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        if let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(name.as_bytes()),
            HeaderValue::from_str(value),
        ) {
            self.headers.insert(name, value);
        }
        self
    }

    pub fn body(mut self, body: Vec<u8>) -> Self {
        self.body = Bytes::from(body);
        self
    }

    pub fn build(self) -> HttpResponseBuilder<Complete> {
        HttpResponseBuilder {
            status: self.status,
            headers: self.headers,
            body: self.body,
            _state: PhantomData,
        }
    }
}

impl HttpResponseBuilder<Complete> {
    pub fn finish(self) -> HttpResponse {
        let status = self.status.unwrap_or(StatusCode::OK);

        // Build raw bytes for forwarding
        let mut raw = Vec::new();
        raw.extend_from_slice(b"HTTP/1.1 ");
        raw.extend_from_slice(status.as_str().as_bytes());
        raw.extend_from_slice(b" ");
        raw.extend_from_slice(status.canonical_reason().unwrap_or("").as_bytes());
        raw.extend_from_slice(b"\r\n");

        for (name, value) in &self.headers {
            raw.extend_from_slice(name.as_str().as_bytes());
            raw.extend_from_slice(b": ");
            raw.extend_from_slice(value.as_bytes());
            raw.extend_from_slice(b"\r\n");
        }
        raw.extend_from_slice(b"\r\n");
        raw.extend_from_slice(&self.body);

        HttpResponse {
            status,
            headers: self.headers,
            body: self.body,
            raw_bytes: Bytes::from(raw),
        }
    }
}

impl Default for HttpResponseBuilder<Building> {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 4: Run tests**

```bash
cargo test --test response_test
```

Expected: PASS

**Step 5: Commit**

```bash
git add http-impl/src/response.rs http-impl/tests/response_test.rs
git commit -m "feat(http-impl): implement HttpResponse with builder pattern"
```

---

### Task 4: Implement Basic Auth

**Files:**
- Create: `http-impl/src/auth.rs`
- Create: `http-impl/tests/auth_test.rs`

**Step 1: Write failing test**

File: `http-impl/tests/auth_test.rs`

```rust
use http_impl::{HttpRequest, BasicAuth};

#[test]
fn test_parse_basic_auth() {
    let raw = b"GET / HTTP/1.1\r\nProxy-Authorization: Basic dXNlcjpwYXNz\r\n\r\n";
    let request = HttpRequest::parse(raw).unwrap();

    let auth = request.parse_basic_auth().unwrap().unwrap();
    assert_eq!(auth.username, "user");
    assert_eq!(auth.password, "pass");
}

#[test]
fn test_no_auth() {
    let raw = b"GET / HTTP/1.1\r\n\r\n";
    let request = HttpRequest::parse(raw).unwrap();

    let auth = request.parse_basic_auth().unwrap();
    assert!(auth.is_none());
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test --test auth_test
```

Expected: FAIL with "cannot find type `BasicAuth`"

**Step 3: Implement BasicAuth**

File: `http-impl/src/auth.rs`

```rust
use crate::error::{HttpError, Result};
use crate::request::HttpRequest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicAuth {
    pub username: String,
    pub password: String,
}

impl HttpRequest {
    pub fn parse_basic_auth(&self) -> Result<Option<BasicAuth>> {
        let auth_header = match self.header("Proxy-Authorization") {
            Some(h) => h,
            None => return Ok(None),
        };

        if !auth_header.starts_with("Basic ") {
            return Err(HttpError::AuthError(
                "Invalid authorization scheme".to_string()
            ));
        }

        let encoded = &auth_header[6..]; // Skip "Basic "
        let decoded = base64::decode(encoded)
            .map_err(|e| HttpError::AuthError(format!("Invalid base64: {}", e)))?;

        let decoded_str = String::from_utf8(decoded)
            .map_err(|e| HttpError::AuthError(format!("Invalid UTF-8: {}", e)))?;

        let parts: Vec<&str> = decoded_str.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(HttpError::AuthError(
                "Invalid credentials format".to_string()
            ));
        }

        Ok(Some(BasicAuth {
            username: parts[0].to_string(),
            password: parts[1].to_string(),
        }))
    }
}
```

**Step 4: Run tests**

```bash
cargo test --test auth_test
```

Expected: PASS

**Step 5: Commit**

```bash
git add http-impl/src/auth.rs http-impl/tests/auth_test.rs
git commit -m "feat(http-impl): implement Basic Auth parsing"
```

---

### Task 5: Implement Async Extension

**Files:**
- Create: `http-impl/src/async_ext.rs`
- Create: `http-impl/tests/async_test.rs`

**Step 1: Write failing test**

File: `http-impl/tests/async_test.rs`

```rust
#![cfg(feature = "async")]

use http_impl::{HttpRequest, HttpResponse, HttpResponseBuilder};
use http::{Method, StatusCode};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn test_async_request_from_stream() {
    let data = b"GET /test HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let mut cursor = std::io::Cursor::new(data.to_vec());

    let request = HttpRequest::from_stream(&mut cursor).await.unwrap();

    assert_eq!(request.method(), &Method::GET);
    assert_eq!(request.uri().path(), "/test");
}

#[tokio::test]
async fn test_async_response_write_to_stream() {
    let response = HttpResponseBuilder::new()
        .status(StatusCode::OK)
        .header("Content-Type", "text/plain")
        .body(b"Hello".to_vec())
        .build()
        .finish();

    let mut buffer = Vec::new();
    response.write_to_stream(&mut buffer).await.unwrap();

    assert!(buffer.starts_with(b"HTTP/1.1 200 OK"));
    assert!(buffer.ends_with(b"Hello"));
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test --test async_test --features async
```

Expected: FAIL with "cannot find method `from_stream`"

**Step 3: Implement async extension**

File: `http-impl/src/async_ext.rs`

```rust
use crate::error::Result;
use crate::{HttpRequest, HttpResponse};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

impl HttpRequest {
    /// Read and parse HTTP request from async stream
    pub async fn from_stream<S>(stream: &mut S) -> Result<Self>
    where
        S: AsyncRead + Unpin,
    {
        let mut buffer = Vec::new();
        let mut temp = [0u8; 4096];

        loop {
            let n = stream.read(&mut temp).await?;
            if n == 0 {
                break;
            }
            buffer.extend_from_slice(&temp[..n]);

            // Check if we have complete headers
            if buffer.windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }

        Self::parse(&buffer)
    }
}

impl HttpResponse {
    /// Write HTTP response to async stream
    pub async fn write_to_stream<S>(&self, stream: &mut S) -> Result<()>
    where
        S: AsyncWrite + Unpin,
    {
        stream.write_all(self.raw_bytes()).await?;
        stream.flush().await?;
        Ok(())
    }

    /// Read and parse HTTP response from async stream
    pub async fn from_stream<S>(stream: &mut S) -> Result<Self>
    where
        S: AsyncRead + Unpin,
    {
        let mut buffer = Vec::new();
        let mut temp = [0u8; 4096];

        loop {
            let n = stream.read(&mut temp).await?;
            if n == 0 {
                break;
            }
            buffer.extend_from_slice(&temp[..n]);

            // Check if we have complete headers
            if buffer.windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }

        Self::parse(&buffer)
    }
}
```

**Step 4: Update lib.rs to include async_ext**

File: `http-impl/src/lib.rs` (add at the end)

```rust
#[cfg(feature = "async")]
mod async_ext;
```

**Step 5: Run tests with async feature**

```bash
cargo test --test async_test --features async
```

Expected: PASS

**Step 6: Run all tests**

```bash
cargo test --all-features
```

Expected: All tests PASS

**Step 7: Commit**

```bash
git add http-impl/src/async_ext.rs http-impl/tests/async_test.rs http-impl/src/lib.rs
git commit -m "feat(http-impl): implement async stream extensions"
```

---

### Task 6: Add to Workspace

**Files:**
- Modify: `Cargo.toml` (workspace root)

**Step 1: Add http-impl to workspace members**

```toml
[workspace]
members = [
    "stream",
    "socks5-impl",
    "http-impl",
]
```

**Step 2: Verify workspace**

```bash
cargo build --workspace
```

Expected: All crates compile successfully

**Step 3: Run all tests**

```bash
cargo test --workspace --all-features
```

Expected: All tests PASS

**Step 4: Commit**

```bash
git add Cargo.toml
git commit -m "feat: add http-impl to workspace"
```

---

### Task 7: Documentation and Examples

**Files:**
- Create: `http-impl/examples/parse_request.rs`
- Create: `http-impl/examples/build_response.rs`
- Modify: `http-impl/README.md`

**Step 1: Create parse request example**

File: `http-impl/examples/parse_request.rs`

```rust
use http_impl::HttpRequest;

fn main() {
    let raw = b"GET /api/users HTTP/1.1\r\n\
                Host: example.com\r\n\
                User-Agent: curl/7.68.0\r\n\
                Accept: */*\r\n\
                \r\n";

    match HttpRequest::parse(raw) {
        Ok(request) => {
            println!("Method: {}", request.method());
            println!("URI: {}", request.uri());
            println!("Headers:");
            for (name, value) in request.headers() {
                println!("  {}: {:?}", name, value);
            }
        }
        Err(e) => {
            eprintln!("Parse error: {}", e);
        }
    }
}
```

**Step 2: Create build response example**

File: `http-impl/examples/build_response.rs`

```rust
use http_impl::HttpResponseBuilder;
use http::StatusCode;

fn main() {
    let response = HttpResponseBuilder::new()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("Content-Length", "13")
        .body(br#"{"ok":true}"#.to_vec())
        .build()
        .finish();

    println!("Response:");
    println!("{}", String::from_utf8_lossy(response.raw_bytes()));
}
```

**Step 3: Run examples**

```bash
cargo run --example parse_request
cargo run --example build_response
```

Expected: Examples run successfully

**Step 4: Update README with more examples**

Append to `http-impl/README.md`:

```markdown
## Examples

### Parse HTTP Request

```rust
use http_impl::HttpRequest;

let raw = b"GET /api/users HTTP/1.1\r\nHost: example.com\r\n\r\n";
let request = HttpRequest::parse(raw)?;

println!("Method: {}", request.method());
println!("URI: {}", request.uri());
```

### Build HTTP Response

```rust
use http_impl::HttpResponseBuilder;
use http::StatusCode;

let response = HttpResponseBuilder::new()
    .status(StatusCode::OK)
    .header("Content-Type", "application/json")
    .body(br#"{"ok":true}"#.to_vec())
    .build()
    .finish();
```

### Async Usage

```rust
use http_impl::HttpRequest;
use tokio::net::TcpStream;

let mut stream = TcpStream::connect("example.com:80").await?;
let request = HttpRequest::from_stream(&mut stream).await?;
```

### Basic Auth

```rust
let request = HttpRequest::parse(b"GET / HTTP/1.1\r\nProxy-Authorization: Basic dXNlcjpwYXNz\r\n\r\n")?;

if let Some(auth) = request.parse_basic_auth()? {
    println!("User: {}", auth.username);
    println!("Pass: {}", auth.password);
}
```
```

**Step 5: Commit**

```bash
git add http-impl/examples/ http-impl/README.md
git commit -m "docs(http-impl): add examples and usage documentation"
```

---

## Verification

After completing all tasks:

```bash
# Build everything
cargo build --workspace --all-features

# Run all tests
cargo test --workspace --all-features

# Check documentation
cargo doc --workspace --all-features --no-deps --open

# Run clippy
cargo clippy --workspace --all-features -- -D warnings

# Check formatting
cargo fmt --all -- --check
```

All commands should succeed.

---

## Integration Notes

This crate is designed to be used in proxy servers like `socks5-impl` or `resigame-gateway-rust`. To integrate:

1. Add dependency in consumer crate:
   ```toml
   [dependencies]
   http-impl = { path = "../http-impl", features = ["async"] }
   ```

2. Replace manual HTTP parsing with http-impl:
   ```rust
   use http_impl::{HttpRequest, HttpResponse};

   // Old: manual httparse usage
   // New: use http-impl
   let request = HttpRequest::from_stream(&mut stream).await?;
   ```

3. Leverage zero-copy for proxying:
   ```rust
   // Forward original bytes without re-serialization
   upstream.write_all(request.raw_bytes()).await?;
   ```
