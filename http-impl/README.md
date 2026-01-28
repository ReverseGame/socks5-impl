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
