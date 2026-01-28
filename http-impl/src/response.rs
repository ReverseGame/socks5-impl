use crate::error::{HttpError, Result};
use bytes::Bytes;
use http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use std::marker::PhantomData;

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
