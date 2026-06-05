//! Cloudflare Workers (`wasm32`) HTTP `GET` transport for **binary** responses
//! (e.g. Places photo bytes). The `worker`-feature counterpart to
//! [`super::get_binary_request`]. Same signature, so place-photo `execute()`
//! delegates here unchanged.

use crate::Error;
use worker::{Fetch, Method};

use super::worker_transport::{build_headers, build_request};

impl crate::Client {
    /// Performs the HTTP `GET` request via [`worker::Fetch`] and returns the raw
    /// response body bytes.
    ///
    /// # Errors
    ///
    /// Fails if the request fails validation, the URL cannot be built, the
    /// `fetch` call fails, or the server returns a non-success status.
    pub(crate) async fn get_binary_request<REQ>(
        &self,
        request: REQ,
    ) -> Result<Vec<u8>, Error>
    where
        REQ: std::fmt::Debug
            + crate::traits::EndPoint
            + crate::traits::QueryUrl
            + crate::traits::RequestHeaders
            + Send,
    {
        let url: String = request
            .query_url()?
            .trim_matches('?')
            .to_string();

        let api_key = REQ::send_x_goog_api_key().then_some(self.key.as_str());
        let headers = build_headers(&request.request_headers(), api_key)?;
        let http_request = build_request(Method::Get, &url, headers, None)?;

        let mut response = Fetch::Request(http_request)
            .send()
            .await
            .map_err(|error| Error::Worker(error.to_string()))?;

        let status = response.status_code();
        if !(200..300).contains(&status) {
            let body = response.text().await.unwrap_or_default();
            return Err(Error::WorkerHttp { status, body });
        }

        response
            .bytes()
            .await
            .map_err(|error| Error::Worker(error.to_string()))
    } // fn
} // impl
