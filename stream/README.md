# stream

A `TcpStream` wrapper with async graceful shutdown on drop.

## Features

- Transparent wrapper around `tokio::net::TcpStream`
- Automatic graceful shutdown on drop (using `tokio::spawn`)
- Implements `AsyncRead`, `AsyncWrite`, `Deref`, and `DerefMut`
- Full access to TCP socket options

## Usage

```rust,no_run
use stream::Stream;
use tokio::net::TcpStream;

# #[tokio::main]
# async fn main() -> std::io::Result<()> {
let tcp_stream = TcpStream::connect("127.0.0.1:8080").await?;
let mut stream = Stream::new(tcp_stream);

// Use as TcpStream via Deref
stream.set_nodelay(true)?;

// Explicit shutdown (recommended for critical connections)
stream.shutdown().await?;

// Or let it drop and shutdown happens in background
# Ok(())
# }
```

## Design

The `Stream` type wraps `Option<TcpStream>` internally to enable async drop:

1. On drop, the inner `TcpStream` is extracted
2. A background task is spawned with `tokio::spawn`
3. Graceful shutdown happens asynchronously

For guaranteed shutdown completion, call `shutdown()` explicitly before dropping.

## License

GPL-3.0-or-later
