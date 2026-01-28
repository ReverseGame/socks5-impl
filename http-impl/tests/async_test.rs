#![cfg(feature = "async")]

use http::{Method, StatusCode};
use http_impl::{HttpRequest, HttpResponseBuilder};

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
        .build();

    let mut buffer = Vec::new();
    response.write_to_stream(&mut buffer).await.unwrap();

    assert!(buffer.starts_with(b"HTTP/1.1 200 OK"));
    assert!(buffer.ends_with(b"Hello"));
}
