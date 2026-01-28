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
    .build();

// Parse from bytes
let request = HttpRequest::parse(b"GET / HTTP/1.1\r\n\r\n").unwrap();
```

## Examples

### Parse HTTP Request

```rust
use http_impl::HttpRequest;

let raw = b"GET /api/users HTTP/1.1\r\nHost: example.com\r\n\r\n";
let request = HttpRequest::parse(raw).unwrap();

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
    .build();
```

### Success Responses

```rust
use http_impl::HttpResponse;

// 200 OK (empty)
let ok = HttpResponse::ok();

// 200 OK with body
let ok_body = HttpResponse::ok_with_body(b"Success".to_vec(), "text/plain");

// 201 Created
let created = HttpResponse::created();

// 204 No Content
let no_content = HttpResponse::no_content();
```

### Common Error Responses

```rust
use http_impl::HttpResponse;

// 400 Bad Request
let bad_req = HttpResponse::bad_request("Invalid request format");

// 401 Unauthorized
let unauth = HttpResponse::unauthorized("Protected Area");

// 403 Forbidden
let forbidden = HttpResponse::forbidden("Access denied");

// 404 Not Found
let not_found = HttpResponse::not_found("Resource not found");

// 407 Proxy Authentication Required (useful for proxy servers)
let proxy_auth = HttpResponse::proxy_auth_required("Proxy");

// 500 Internal Server Error
let server_err = HttpResponse::internal_server_error("Something went wrong");

// 502 Bad Gateway
let bad_gw = HttpResponse::bad_gateway("Upstream server error");

// 503 Service Unavailable
let unavail = HttpResponse::service_unavailable("Service temporarily down");
```

### Async Usage

```rust,ignore
# #[tokio::main]
# async fn main() -> Result<(), Box<dyn std::error::Error>> {
use http_impl::HttpRequest;
use tokio::net::TcpStream;

let mut stream = TcpStream::connect("example.com:80").await?;
let request = HttpRequest::from_stream(&mut stream).await?;
# Ok(())
# }
```

### Basic Auth

```rust
use http_impl::HttpRequest;

let request = HttpRequest::parse(b"GET / HTTP/1.1\r\nProxy-Authorization: Basic dXNlcjpwYXNz\r\n\r\n").unwrap();

if let Some(auth) = request.parse_basic_auth().unwrap() {
    println!("User: {}", auth.username);
    println!("Pass: {}", auth.password);
}
```

### Pre-built Response Constants

For high-performance scenarios, use pre-built byte constants that can be sent directly to streams:

```rust,ignore
use http_impl::constants;

// Direct send to stream (sync)
stream.write_all(constants::SUCCESS)?;

// Or with async
stream.write_all(constants::AUTHENTICATION_REQUIRED).await?;
```

Available constants:
- `SUCCESS` - HTTP/1.1 200 OK
- `UNAUTHORIZED` - HTTP/1.1 401 Unauthorized
- `FORBIDDEN` - HTTP/1.1 403 Forbidden
- `AUTHENTICATION_REQUIRED` - HTTP/1.1 407 Proxy Authentication Required
- `BAD_REQUEST` - HTTP/1.1 400 Bad Request
- `NOT_FOUND` - HTTP/1.1 404 Not Found
- `INTERNAL_SERVER_ERROR` - HTTP/1.1 500 Internal Server Error
- `BAD_GATEWAY` - HTTP/1.1 502 Bad Gateway
- `SERVICE_UNAVAILABLE` - HTTP/1.1 503 Service Unavailable

## License

GPL-3.0-or-later
