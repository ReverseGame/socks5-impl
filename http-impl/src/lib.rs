#![doc = include_str!("../README.md")]

pub mod error;
pub mod request;
pub mod response;
pub mod auth;

#[cfg(feature = "async")]
pub mod async_ext;

pub use error::{HttpError, Result};
pub use request::{HttpRequest, HttpRequestBuilder};
pub use response::{HttpResponse, HttpResponseBuilder};
pub use auth::BasicAuth;
