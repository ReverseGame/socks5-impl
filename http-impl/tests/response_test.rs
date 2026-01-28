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

#[test]
fn test_error_responses() {
    // Test 400 Bad Request
    let resp = HttpResponse::bad_request("Invalid request format");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    assert_eq!(resp.body().as_ref(), b"Invalid request format");
    assert_eq!(resp.header("Content-Type").unwrap(), "text/plain; charset=utf-8");

    // Test 401 Unauthorized
    let resp = HttpResponse::unauthorized("Protected Area");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(resp.header("WWW-Authenticate").unwrap(), "Basic realm=\"Protected Area\"");

    // Test 403 Forbidden
    let resp = HttpResponse::forbidden("Access denied");
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    assert_eq!(resp.body().as_ref(), b"Access denied");

    // Test 404 Not Found
    let resp = HttpResponse::not_found("Resource not found");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    assert_eq!(resp.body().as_ref(), b"Resource not found");

    // Test 407 Proxy Authentication Required
    let resp = HttpResponse::proxy_auth_required("Proxy");
    assert_eq!(resp.status(), StatusCode::PROXY_AUTHENTICATION_REQUIRED);
    assert_eq!(resp.header("Proxy-Authenticate").unwrap(), "Basic realm=\"Proxy\"");

    // Test 500 Internal Server Error
    let resp = HttpResponse::internal_server_error("Server error");
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(resp.body().as_ref(), b"Server error");

    // Test 502 Bad Gateway
    let resp = HttpResponse::bad_gateway("Upstream server error");
    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
    assert_eq!(resp.body().as_ref(), b"Upstream server error");

    // Test 503 Service Unavailable
    let resp = HttpResponse::service_unavailable("Temporarily unavailable");
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(resp.body().as_ref(), b"Temporarily unavailable");
}
