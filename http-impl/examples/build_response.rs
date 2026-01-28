use http_impl::HttpResponseBuilder;
use http::StatusCode;

fn main() {
    let response = HttpResponseBuilder::new()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("Content-Length", "13")
        .body(br#"{"ok":true}"#.to_vec())
        .build();

    println!("Response:");
    println!("{}", String::from_utf8_lossy(response.raw_bytes()));
}
