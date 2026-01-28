use crate::error::{HttpError, Result};
use crate::{HttpRequest, HttpResponse};
use bytes::BytesMut;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

const MAX_HEADER_SIZE: usize = 1024 * 32;
const BUFFER_SIZE: usize = 4096;
const CRLF2: &[u8] = b"\r\n\r\n";

impl HttpRequest {
    /// Read and parse HTTP request from async stream
    pub async fn from_stream<S>(stream: &mut S) -> Result<Self>
    where
        S: AsyncRead + Unpin,
    {
        let mut buffer = BytesMut::with_capacity(BUFFER_SIZE);
        let mut temp = [0u8; BUFFER_SIZE];

        loop {
            let n = stream.read(&mut temp).await?;
            if n == 0 {
                return Err(HttpError::InvalidRequest("Connection closed".to_string()));
            }

            let prev_len = buffer.len();
            buffer.extend_from_slice(&temp[..n]);

            if buffer.len() > MAX_HEADER_SIZE {
                return Err(HttpError::InvalidRequest("Header too large".to_string()));
            }

            // Fast path: check last 4 bytes first
            if buffer.len() >= 4 && &buffer[buffer.len() - 4..] == CRLF2 {
                break;
            }

            // Check overlap region (previous read boundary)
            let search_start = if prev_len >= 3 { prev_len - 3 } else { 0 };
            if memchr::memmem::find(&buffer[search_start..], CRLF2).is_some() {
                break;
            }
        }

        Self::parse_bytes(buffer.freeze())
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
        let mut buffer = BytesMut::with_capacity(BUFFER_SIZE);
        let mut temp = [0u8; BUFFER_SIZE];

        loop {
            let n = stream.read(&mut temp).await?;
            if n == 0 {
                return Err(HttpError::InvalidResponse("Connection closed".to_string()));
            }

            let prev_len = buffer.len();
            buffer.extend_from_slice(&temp[..n]);

            if buffer.len() > MAX_HEADER_SIZE {
                return Err(HttpError::InvalidResponse("Header too large".to_string()));
            }

            // Fast path: check last 4 bytes first
            if buffer.len() >= 4 && &buffer[buffer.len() - 4..] == CRLF2 {
                break;
            }

            // Check overlap region (previous read boundary)
            let search_start = if prev_len >= 3 { prev_len - 3 } else { 0 };
            if memchr::memmem::find(&buffer[search_start..], CRLF2).is_some() {
                break;
            }
        }

        Self::parse_bytes(buffer.freeze())
    }
}
