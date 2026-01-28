#![doc = include_str!("../README.md")]

pub mod auth;
pub mod error;
pub mod request;
pub mod response;

#[cfg(feature = "async")]
pub mod async_ext;

pub use auth::BasicAuth;
pub use error::{HttpError, Result};
pub use request::{HttpRequest, HttpRequestBuilder};
pub use response::{constants, HttpResponse, HttpResponseBuilder};
