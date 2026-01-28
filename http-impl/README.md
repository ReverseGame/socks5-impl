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
let request = HttpRequest::parse(b"GET / HTTP/1.1\r\n\r\n").unwrap();
```

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

## License

GPL-3.0-or-later
