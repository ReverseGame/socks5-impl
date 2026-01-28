#![doc = include_str!("../README.md")]

pub mod error;
pub mod request;
// TODO: Uncomment in Task 3
// pub mod response;
// TODO: Uncomment in Task 4
// pub mod auth;

#[cfg(feature = "async")]
pub mod async_ext;

pub use error::{HttpError, Result};
pub use request::{HttpRequest, HttpRequestBuilder};
// TODO: Uncomment in Task 3
// pub use response::{HttpResponse, HttpResponseBuilder};
// TODO: Uncomment in Task 4
// pub use auth::BasicAuth;
