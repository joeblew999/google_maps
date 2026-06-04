//! Shared helpers for the Cloudflare Workers (`wasm32`) HTTP transport.
//!
//! These back the `worker`-feature counterparts of the `reqwest`-based
//! [`super::get_request`] / [`super::post_request`] methods. They use
//! [`worker::Fetch`] and deliberately omit the tokio-based rate limiter and
//! `backon` retry — neither works inside a Cloudflare Worker (no timer).

#![allow(dead_code, reason = "individual helpers are used by whichever of the GET/POST transports the active feature set compiles")]

use crate::Error;
use worker::{Fetch, Headers, Method, Request, RequestInit};

// -------------------------------------------------------------------------------------------------
//
/// Builds a [`worker::Headers`] from an [`http::HeaderMap`], optionally adding
/// the sensitive `X-Goog-Api-Key` header (used by the Places API "New").
pub(crate) fn build_headers(
    map: &http::HeaderMap,
    api_key: Option<&str>,
) -> Result<Headers, Error> {
    let headers = Headers::new();

    for (name, value) in map {
        let value = value.to_str().map_err(|_error| Error::InvalidHeaderValue {
            header_name: name.as_str().to_string(),
        })?;
        headers
            .set(name.as_str(), value)
            .map_err(|error| Error::Worker(error.to_string()))?;
    } // for

    if let Some(key) = api_key {
        headers
            .set("X-Goog-Api-Key", key)
            .map_err(|error| Error::Worker(error.to_string()))?;
    } // if

    Ok(headers)
} // fn

// -------------------------------------------------------------------------------------------------
//
/// Builds a [`worker::Request`] with the given method, URL, headers, and an
/// optional request body.
pub(crate) fn build_request(
    method: Method,
    url: &str,
    headers: Headers,
    body: Option<String>,
) -> Result<Request, Error> {
    let mut init = RequestInit::new();
    init.with_method(method).with_headers(headers);

    if let Some(body) = body {
        init.with_body(Some(body.into()));
    } // if

    Request::new_with_init(url, &init).map_err(|error| Error::Worker(error.to_string()))
} // fn

// -------------------------------------------------------------------------------------------------
//
/// Sends a prepared [`worker::Request`], checks the HTTP status, and
/// deserializes the JSON body into `RSP` — surfacing any API-level error
/// embedded in a `2xx` body via the `Into<Result<RSP, ERR>>` conversion.
pub(crate) async fn send<RSP, ERR>(request: Request) -> Result<RSP, Error>
where
    RSP: serde::de::DeserializeOwned + Into<Result<RSP, ERR>>,
    ERR: std::fmt::Display + Into<Error>,
{
    let mut response = Fetch::Request(request)
        .send()
        .await
        .map_err(|error| Error::Worker(error.to_string()))?;

    let status = response.status_code();

    let body = response
        .text()
        .await
        .map_err(|error| Error::Worker(error.to_string()))?;

    if !(200..300).contains(&status) {
        return Err(Error::WorkerHttp { status, body });
    } // if

    let deserialized: RSP = serde_json::from_str(&body)?;
    let result: Result<RSP, ERR> = deserialized.into();
    result.map_err(Into::into)
} // fn
