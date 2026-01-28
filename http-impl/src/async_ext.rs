use crate::error::Result;
use crate::{HttpRequest, HttpResponse};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

impl HttpRequest {
    /// Read and parse HTTP request from async stream
    pub async fn from_stream<S>(stream: &mut S) -> Result<Self>
    where
        S: AsyncRead + Unpin,
    {
        let mut buffer = Vec::new();
        let mut temp = [0u8; 4096];

        loop {
            let n = stream.read(&mut temp).await?;
            if n == 0 {
                break;
            }
            buffer.extend_from_slice(&temp[..n]);

            // Check if we have complete headers
            if buffer.windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }

        Self::parse(&buffer)
    }
}

impl HttpResponse {
    /// Write HTTP response to async stream
    pub async fn write_to_stream<S>(&self, stream: &mut S) -> Result<()>
    where
        S: AsyncWrite + Unpin,
    {
        stream.write_all(self.raw_bytes()).await?;
        stream.flush().await?;
        Ok(())
    }

    /// Read and parse HTTP response from async stream
    pub async fn from_stream<S>(stream: &mut S) -> Result<Self>
    where
        S: AsyncRead + Unpin,
    {
        let mut buffer = Vec::new();
        let mut temp = [0u8; 4096];

        loop {
            let n = stream.read(&mut temp).await?;
            if n == 0 {
                break;
            }
            buffer.extend_from_slice(&temp[..n]);

            // Check if we have complete headers
            if buffer.windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }

        Self::parse(&buffer)
    }
}
