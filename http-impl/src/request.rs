use crate::error::{HttpError, Result};
use bytes::{Bytes, BytesMut};
use http::{HeaderMap, HeaderName, HeaderValue, Method, Uri};

/// HTTP request with zero-copy body and preserved raw bytes
#[derive(Debug, Clone)]
pub struct HttpRequest {
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
    raw_bytes: Bytes,
}

impl HttpRequest {
    /// Parse HTTP request from Bytes (zero-copy)
    /// This is more efficient when data is already in Bytes format
    pub fn parse_bytes(data: Bytes) -> Result<Self> {
        let mut headers_buf = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers_buf);

        let status = req.parse(&data).map_err(|e| HttpError::InvalidRequest(e.to_string()))?;

        let header_len = match status {
            httparse::Status::Complete(len) => len,
            httparse::Status::Partial => return Err(HttpError::InvalidRequest("Incomplete request".to_string())),
        };

        // Parse method
        let method = req.method.ok_or_else(|| HttpError::InvalidRequest("Missing method".to_string()))?;
        let method = method.parse::<Method>().map_err(|e| HttpError::InvalidRequest(e.to_string()))?;

        // Parse URI
        let path = req.path.ok_or_else(|| HttpError::InvalidRequest("Missing path".to_string()))?;
        let uri = path.parse::<Uri>().map_err(|e| HttpError::InvalidUri(e.to_string()))?;

        // Parse headers
        let mut headers = HeaderMap::new();
        for header in req.headers {
            let name = HeaderName::from_bytes(header.name.as_bytes()).map_err(|e| HttpError::InvalidHeader(e.to_string()))?;
            let value = HeaderValue::from_bytes(header.value).map_err(|e| HttpError::InvalidHeader(e.to_string()))?;
            headers.insert(name, value);
        }

        // Zero-copy: slice the original Bytes
        let body = data.slice(header_len..);
        let raw_bytes = data;

        Ok(Self {
            method,
            uri,
            headers,
            body,
            raw_bytes,
        })
    }

    /// Parse HTTP request from raw bytes
    pub fn parse(data: &[u8]) -> Result<Self> {
        // Convert to Bytes and use parse_bytes
        Self::parse_bytes(Bytes::copy_from_slice(data))
    }

    /// Get HTTP method
    pub fn method(&self) -> &Method {
        &self.method
    }

    /// Get URI
    pub fn uri(&self) -> &Uri {
        &self.uri
    }

    /// Get all headers
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Get specific header value
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }

    /// Get request body
    pub fn body(&self) -> &Bytes {
        &self.body
    }

    /// Get raw request bytes (for forwarding)
    pub fn raw_bytes(&self) -> &Bytes {
        &self.raw_bytes
    }
}

/// Builder for HttpRequest
pub struct HttpRequestBuilder {
    method: Option<Method>,
    uri: Option<Uri>,
    headers: HeaderMap,
    body: Bytes,
}

impl HttpRequestBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            method: None,
            uri: None,
            headers: HeaderMap::new(),
            body: Bytes::new(),
        }
    }

    /// Set HTTP method
    pub fn method(mut self, method: Method) -> Self {
        self.method = Some(method);
        self
    }

    /// Set URI
    pub fn uri(mut self, uri: Uri) -> Self {
        self.uri = Some(uri);
        self
    }

    /// Add header
    pub fn header(mut self, name: &str, value: &str) -> Self {
        if let (Ok(name), Ok(value)) = (HeaderName::from_bytes(name.as_bytes()), HeaderValue::from_str(value)) {
            self.headers.insert(name, value);
        }
        self
    }

    /// Set body (accepts Vec<u8>, Bytes, &[u8], String, etc.)
    pub fn body(mut self, body: impl Into<Bytes>) -> Self {
        self.body = body.into();
        self
    }

    /// Build and create HttpRequest
    pub fn build(self) -> HttpRequest {
        let method = self.method.unwrap_or(Method::GET);
        let uri = self.uri.unwrap_or_else(|| "/".parse().unwrap());

        // Build raw bytes using BytesMut for zero-copy conversion
        let mut raw = BytesMut::new();
        raw.extend_from_slice(method.as_str().as_bytes());
        raw.extend_from_slice(b" ");
        raw.extend_from_slice(uri.path().as_bytes());
        raw.extend_from_slice(b" HTTP/1.1\r\n");

        for (name, value) in self.headers.iter() {
            raw.extend_from_slice(name.as_str().as_bytes());
            raw.extend_from_slice(b": ");
            raw.extend_from_slice(value.as_bytes());
            raw.extend_from_slice(b"\r\n");
        }

        raw.extend_from_slice(b"\r\n");
        raw.extend_from_slice(&self.body);

        HttpRequest {
            method,
            uri,
            headers: self.headers,
            body: self.body,
            raw_bytes: raw.freeze(),
        }
    }
}

impl Default for HttpRequestBuilder {
    fn default() -> Self {
        Self::new()
    }
}
