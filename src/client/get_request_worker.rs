//! Cloudflare Workers (`wasm32`) HTTP `GET` transport.
//!
//! The `worker`-feature counterpart to [`super::get_request`] (which uses
//! `reqwest`). Same signature, same generic bounds — so every per-API
//! `execute()` method delegates here unchanged.

use crate::Error;
use worker::Method;

use super::worker_transport::{build_headers, build_request, send};

impl crate::Client {
    /// Performs the HTTP `GET` request via [`worker::Fetch`] and returns the
    /// deserialized response to the caller.
    ///
    /// # Errors
    ///
    /// This method can fail if the request fails validation, the URL cannot be
    /// built, the `fetch` call fails, the server returns a non-success status,
    /// or the response body cannot be deserialized.
    pub(crate) async fn get_request<REQ, RSP, ERR>(
        &self,
        request: REQ,
    ) -> Result<RSP, Error>
    where
        REQ: std::fmt::Debug
            + crate::traits::EndPoint
            + crate::traits::QueryUrl
            + crate::traits::RequestHeaders
            + Send,
        RSP: serde::de::DeserializeOwned + Into<Result<RSP, ERR>>,
        ERR: std::fmt::Display + From<ERR> + Into<Error>,
    {
        let url: String = request
            .query_url()?
            .trim_matches('?')
            .to_string();

        let api_key = REQ::send_x_goog_api_key().then_some(self.key.as_str());
        let headers = build_headers(&request.request_headers(), api_key)?;
        let http_request = build_request(Method::Get, &url, headers, None)?;

        send::<RSP, ERR>(http_request).await
    } // fn
} // impl
