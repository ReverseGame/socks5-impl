use http_impl::HttpRequest;

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
