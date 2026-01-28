use crate::error::{HttpError, Result};
use bytes::Bytes;
use http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use std::marker::PhantomData;

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

    /// Create a 200 OK response
    pub fn ok() -> Self {
        HttpResponseBuilder::new()
            .status(StatusCode::OK)
            .build()
            .finish()
    }

    /// Create a 200 OK response with body
    pub fn ok_with_body(body: impl Into<Vec<u8>>, content_type: &str) -> Self {
        HttpResponseBuilder::new()
            .status(StatusCode::OK)
            .header("Content-Type", content_type)
            .body(body.into())
            .build()
            .finish()
    }

    /// Create a 201 Created response
    pub fn created() -> Self {
        HttpResponseBuilder::new()
            .status(StatusCode::CREATED)
            .build()
            .finish()
    }

    /// Create a 204 No Content response
    pub fn no_content() -> Self {
        HttpResponseBuilder::new()
            .status(StatusCode::NO_CONTENT)
            .build()
            .finish()
    }

    /// Create a 400 Bad Request response
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::error_response(StatusCode::BAD_REQUEST, message)
    }

    /// Create a 401 Unauthorized response with WWW-Authenticate header
    pub fn unauthorized(realm: &str) -> Self {
        HttpResponseBuilder::new()
            .status(StatusCode::UNAUTHORIZED)
            .header("WWW-Authenticate", &format!("Basic realm=\"{}\"", realm))
            .body(b"401 Unauthorized".to_vec())
            .build()
            .finish()
    }

    /// Create a 403 Forbidden response
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::error_response(StatusCode::FORBIDDEN, message)
    }

    /// Create a 404 Not Found response
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::error_response(StatusCode::NOT_FOUND, message)
    }

    /// Create a 407 Proxy Authentication Required response
    pub fn proxy_auth_required(realm: &str) -> Self {
        HttpResponseBuilder::new()
            .status(StatusCode::PROXY_AUTHENTICATION_REQUIRED)
            .header("Proxy-Authenticate", &format!("Basic realm=\"{}\"", realm))
            .body(b"407 Proxy Authentication Required".to_vec())
            .build()
            .finish()
    }

    /// Create a 500 Internal Server Error response
    pub fn internal_server_error(message: impl Into<String>) -> Self {
        Self::error_response(StatusCode::INTERNAL_SERVER_ERROR, message)
    }

    /// Create a 502 Bad Gateway response
    pub fn bad_gateway(message: impl Into<String>) -> Self {
        Self::error_response(StatusCode::BAD_GATEWAY, message)
    }

    /// Create a 503 Service Unavailable response
    pub fn service_unavailable(message: impl Into<String>) -> Self {
        Self::error_response(StatusCode::SERVICE_UNAVAILABLE, message)
    }

    /// Helper function to create error responses with a message
    fn error_response(status: StatusCode, message: impl Into<String>) -> Self {
        let msg = message.into();
        HttpResponseBuilder::new()
            .status(status)
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(msg.into_bytes())
            .build()
            .finish()
    }
}

// Builder pattern with type-state
pub struct HttpResponseBuilder<S> {
    status: Option<StatusCode>,
    headers: HeaderMap,
    body: Bytes,
    _state: PhantomData<S>,
}

pub struct Building;
pub struct Complete;

impl HttpResponseBuilder<Building> {
    pub fn new() -> Self {
        Self {
            status: None,
            headers: HeaderMap::new(),
            body: Bytes::new(),
            _state: PhantomData,
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

    pub fn build(self) -> HttpResponseBuilder<Complete> {
        HttpResponseBuilder {
            status: self.status,
            headers: self.headers,
            body: self.body,
            _state: PhantomData,
        }
    }
}

impl HttpResponseBuilder<Complete> {
    pub fn finish(self) -> HttpResponse {
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

impl Default for HttpResponseBuilder<Building> {
    fn default() -> Self {
        Self::new()
    }
}
