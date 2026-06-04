//! Contains `trait` declarations that are used internally in the `google_maps` crate.
//!
//! These are used for validating and converting request structs into HTTP queries.

pub mod classifiable_error;
pub use crate::traits::classifiable_error::ClassifiableError;

pub mod end_point;
pub use crate::traits::end_point::EndPoint;

pub mod query_string;
pub use crate::traits::query_string::QueryString;

pub mod query_url;
pub use crate::traits::query_url::QueryUrl;

pub mod request_body;
pub use crate::traits::request_body::RequestBody;

pub mod validatable;
pub use crate::traits::validatable::Validatable;

#[cfg(any(feature = "reqwest", feature = "worker"))]
pub mod request_headers;
#[cfg(any(feature = "reqwest", feature = "worker"))]
pub use crate::traits::request_headers::RequestHeaders;