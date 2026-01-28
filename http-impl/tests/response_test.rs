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
        .build();

    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(response.body().as_ref(), b"{}");
}

#[test]
fn test_parse_status() {
    // Test simple 200 OK
    let status = HttpResponse::parse_status(b"HTTP/1.1 200 OK\r\n\r\n").unwrap();
    assert_eq!(status, 200);

    // Test 404 Not Found
    let status = HttpResponse::parse_status(b"HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\n\r\nNot found").unwrap();
    assert_eq!(status, 404);

    // Test 500 Internal Server Error
    let status = HttpResponse::parse_status(b"HTTP/1.1 500 Internal Server Error\r\n\r\n").unwrap();
    assert_eq!(status, 500);

    // Test partial response (only status line)
    let status = HttpResponse::parse_status(b"HTTP/1.1 201 Created\r\n").unwrap();
    assert_eq!(status, 201);
}
