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
        .build();

    assert_eq!(request.method(), &Method::POST);
    assert_eq!(request.body().as_ref(), b"{}");
}
