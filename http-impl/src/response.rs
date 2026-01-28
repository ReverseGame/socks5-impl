use crate::error::{HttpError, Result};
use bytes::Bytes;
use http::{HeaderMap, HeaderName, HeaderValue, StatusCode};

/// Pre-built HTTP response constants for high-performance scenarios
/// These can be sent directly without constructing response objects
pub mod constants {
    /// HTTP/1.1 200 OK with empty body
    pub const SUCCESS: &[u8] = b"HTTP/1.1 200 OK\r\n\r\n";

    /// HTTP/1.1 401 Unauthorized
    pub const UNAUTHORIZED: &[u8] = b"HTTP/1.1 401 Unauthorized\r\n\r\nUnauthorized\r\n";

    /// HTTP/1.1 403 Forbidden
    pub const FORBIDDEN: &[u8] = b"HTTP/1.1 403 Forbidden\r\n\r\n";

    /// HTTP/1.1 407 Proxy Authentication Required with Basic realm
    pub const AUTHENTICATION_REQUIRED: &[u8] = b"HTTP/1.1 407 Proxy Authentication Required\r\nProxy-Authenticate: Basic realm=\"Proxy-Login\"\r\n\r\n";

    /// HTTP/1.1 400 Bad Request
    pub const BAD_REQUEST: &[u8] = b"HTTP/1.1 400 Bad Request\r\n\r\n";

    /// HTTP/1.1 404 Not Found
    pub const NOT_FOUND: &[u8] = b"HTTP/1.1 404 Not Found\r\n\r\n";

    /// HTTP/1.1 500 Internal Server Error
    pub const INTERNAL_SERVER_ERROR: &[u8] = b"HTTP/1.1 500 Internal Server Error\r\n\r\n";

    /// HTTP/1.1 502 Bad Gateway
    pub const BAD_GATEWAY: &[u8] = b"HTTP/1.1 502 Bad Gateway\r\n\r\n";

    /// HTTP/1.1 503 Service Unavailable
    pub const SERVICE_UNAVAILABLE: &[u8] = b"HTTP/1.1 503 Service Unavailable\r\n\r\n";
}

pub struct HttpResponse {
    status: StatusCode,
    headers: HeaderMap,
    body: Bytes,
    raw_bytes: Bytes,
}

impl HttpResponse {
    /// Parse only the status code from HTTP response without parsing headers or body
    /// This is faster than full parse when you only need the status code
    pub fn parse_status(data: &[u8]) -> Result<StatusCode> {
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut resp = httparse::Response::new(&mut headers);

        resp.parse(data)
            .map_err(|e| HttpError::InvalidResponse(e.to_string()))?;

        let status_code = resp.code
            .ok_or_else(|| HttpError::InvalidResponse("Missing status code".to_string()))?;

        StatusCode::from_u16(status_code)
            .map_err(|e| HttpError::InvalidResponse(e.to_string()))
    }

    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut resp = httparse::Response::new(&mut headers);

        let status = resp.parse(data)
            .map_err(|e| HttpError::InvalidResponse(e.to_string()))?;

        let header_len = match status {
            httparse::Status::Complete(len) => len,
            httparse::Status::Partial => {
                return Err(HttpError::InvalidResponse("Incomplete response".to_string()));
            }
        };

        let status_code = resp.code
            .ok_or_else(|| HttpError::InvalidResponse("Missing status code".to_string()))?;

        let status = StatusCode::from_u16(status_code)
            .map_err(|e| HttpError::InvalidResponse(e.to_string()))?;

        let mut header_map = HeaderMap::new();
        for header in resp.headers {
            let name = HeaderName::from_bytes(header.name.as_bytes())
                .map_err(|e| HttpError::InvalidHeader(e.to_string()))?;
            let value = HeaderValue::from_bytes(header.value)
                .map_err(|e| HttpError::InvalidHeader(e.to_string()))?;
            header_map.insert(name, value);
        }

        let body = Bytes::copy_from_slice(&data[header_len..]);
        let raw_bytes = Bytes::copy_from_slice(data);

        Ok(Self {
            status,
            headers: header_map,
            body,
            raw_bytes,
        })
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }

    pub fn body(&self) -> &Bytes {
        &self.body
    }

    pub fn raw_bytes(&self) -> &Bytes {
        &self.raw_bytes
    }
}

/// Builder for HttpResponse
pub struct HttpResponseBuilder {
    status: Option<StatusCode>,
    headers: HeaderMap,
    body: Bytes,
}

impl HttpResponseBuilder {
    pub fn new() -> Self {
        Self {
            status: None,
            headers: HeaderMap::new(),
            body: Bytes::new(),
        }
    }

    pub fn status(mut self, status: StatusCode) -> Self {
        self.status = Some(status);
        self
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        if let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(name.as_bytes()),
            HeaderValue::from_str(value),
        ) {
            self.headers.insert(name, value);
        }
        self
    }

    pub fn body(mut self, body: Vec<u8>) -> Self {
        self.body = Bytes::from(body);
        self
    }

    pub fn build(self) -> HttpResponse {
        let status = self.status.unwrap_or(StatusCode::OK);

        // Build raw bytes for forwarding
        let mut raw = Vec::new();
        raw.extend_from_slice(b"HTTP/1.1 ");
        raw.extend_from_slice(status.as_str().as_bytes());
        raw.extend_from_slice(b" ");
        raw.extend_from_slice(status.canonical_reason().unwrap_or("").as_bytes());
        raw.extend_from_slice(b"\r\n");

        for (name, value) in &self.headers {
            raw.extend_from_slice(name.as_str().as_bytes());
            raw.extend_from_slice(b": ");
            raw.extend_from_slice(value.as_bytes());
            raw.extend_from_slice(b"\r\n");
        }
        raw.extend_from_slice(b"\r\n");
        raw.extend_from_slice(&self.body);

        HttpResponse {
            status,
            headers: self.headers,
            body: self.body,
            raw_bytes: Bytes::from(raw),
        }
    }
}

impl Default for HttpResponseBuilder {
    fn default() -> Self {
        Self::new()
    }
}
