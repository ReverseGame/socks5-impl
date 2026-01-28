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
