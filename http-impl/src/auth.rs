use crate::error::{HttpError, Result};
use crate::request::HttpRequest;
use base64::{Engine as _, engine::general_purpose};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicAuth {
    pub username: String,
    pub password: String,
}

impl HttpRequest {
    pub fn parse_basic_auth(&self) -> Result<Option<BasicAuth>> {
        let auth_header = match self.header("Proxy-Authorization") {
            Some(h) => h,
            None => return Ok(None),
        };

        if !auth_header.starts_with("Basic ") {
            return Err(HttpError::AuthError("Invalid authorization scheme".to_string()));
        }

        let encoded = &auth_header[6..]; // Skip "Basic "
        let decoded = general_purpose::STANDARD
            .decode(encoded)
            .map_err(|e| HttpError::AuthError(format!("Invalid base64: {}", e)))?;

        let decoded_str = String::from_utf8(decoded).map_err(|e| HttpError::AuthError(format!("Invalid UTF-8: {}", e)))?;

        let parts: Vec<&str> = decoded_str.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(HttpError::AuthError("Invalid credentials format".to_string()));
        }

        Ok(Some(BasicAuth {
            username: parts[0].to_string(),
            password: parts[1].to_string(),
        }))
    }
}
