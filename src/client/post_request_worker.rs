//! Cloudflare Workers (`wasm32`) HTTP `POST` transport.
//!
//! The `worker`-feature counterpart to [`super::post_request`] (which uses
//! `reqwest`). Same signature, same generic bounds — so the Places API "New"
//! `execute()` methods delegate here unchanged.

use crate::Error;
use worker::Method;

use super::worker_transport::{build_headers, build_request, send};

impl crate::Client {
    /// Performs the HTTP `POST` request via [`worker::Fetch`] and returns the
    /// deserialized response to the caller.
    ///
    /// # Errors
    ///
    /// This method can fail if the request fails validation, the URL or body
    /// cannot be built, the `fetch` call fails, the server returns a
    /// non-success status, or the response body cannot be deserialized.
    pub(crate) async fn post_request<REQ, RSP, ERR>(
        &self,
        request: REQ,
    ) -> Result<RSP, Error>
    where
        REQ: std::fmt::Debug
            + crate::traits::EndPoint
            + crate::traits::QueryUrl
            + crate::traits::RequestBody
            + crate::traits::RequestHeaders
            + Send,
        RSP: serde::de::DeserializeOwned + Into<Result<RSP, ERR>>,
        ERR: std::fmt::Display + From<ERR> + Into<Error>,
    {
        let url: String = request
            .query_url()?
            .trim_matches('?')
            .to_string();

        let body = request.request_body()?;

        let api_key = REQ::send_x_goog_api_key().then_some(self.key.as_str());
        let headers = build_headers(&request.request_headers(), api_key)?;
        headers
            .set("Content-Type", "application/json")
            .map_err(|error| Error::Worker(error.to_string()))?;

        let http_request = build_request(Method::Post, &url, headers, Some(body))?;

        send::<RSP, ERR>(http_request).await
    } // fn
} // impl
