//! Bearer-token authentication as a `tower::Layer` for the maps Connect service.
//!
//! Wrap a `ConnectRpcService` so the shared maps worker only answers callers that
//! present `Authorization: Bearer <token>` matching an allow-list (one token per
//! consumer; the list comes from a Cloudflare secret, e.g. `MAPS_TOKENS`).
//! Unknown/missing tokens get a Connect `unauthenticated` error — the inner
//! service (and the Google key) is never touched.
//!
//! Target-agnostic: the future stays generic over the inner service's future, so
//! it's `Send` on a native (axum) server and `!Send` on a Worker, just like the
//! service it wraps.
//!
//! ```ignore
//! let layer = TokenAuthLayer::from_list(&env.var("MAPS_TOKENS")?.to_string());
//! let mut svc = layer.layer(ConnectRpcService::new(router));
//! svc.call(req).await
//! ```

use std::collections::HashSet;
use std::convert::Infallible;
use std::sync::Arc;
use std::task::{Context as TaskContext, Poll};

use connectrpc::{ConnectError, ConnectRpcBody};
use http::Response;
use http_body_util::Full;
use pin_project_lite::pin_project;
use tower::{Layer, Service};

/// Layer that rejects requests without a valid `Authorization: Bearer <token>`.
#[derive(Clone)]
pub struct TokenAuthLayer {
    allowed: Arc<HashSet<String>>,
}

impl TokenAuthLayer {
    /// Build from an explicit set of tokens.
    #[must_use]
    pub fn new<I: IntoIterator<Item = String>>(tokens: I) -> Self {
        Self { allowed: Arc::new(tokens.into_iter().collect()) }
    }

    /// Build from a comma/space/newline-separated list (e.g. a CF secret value).
    /// An empty list means **deny all** — the safe default for a shared key.
    #[must_use]
    pub fn from_list(list: &str) -> Self {
        Self::new(
            list.split([',', ' ', '\n', '\t'])
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(String::from),
        )
    }
}

impl<S> Layer<S> for TokenAuthLayer {
    type Service = TokenAuthService<S>;
    fn layer(&self, inner: S) -> Self::Service {
        TokenAuthService { inner, allowed: Arc::clone(&self.allowed) }
    }
}

/// Per-request service produced by [`TokenAuthLayer::layer`].
#[derive(Clone)]
pub struct TokenAuthService<S> {
    inner: S,
    allowed: Arc<HashSet<String>>,
}

impl<S, B> Service<http::Request<B>> for TokenAuthService<S>
where
    S: Service<http::Request<B>, Response = Response<ConnectRpcBody>, Error = Infallible>,
{
    type Response = Response<ConnectRpcBody>;
    type Error = Infallible;
    type Future = TokenAuthFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut TaskContext<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: http::Request<B>) -> Self::Future {
        if token_ok(&req, &self.allowed) {
            TokenAuthFuture::pass(self.inner.call(req))
        } else {
            TokenAuthFuture::denied(deny(ConnectError::unauthenticated(
                "missing or invalid bearer token",
            )))
        }
    }
}

fn token_ok<B>(req: &http::Request<B>, allowed: &HashSet<String>) -> bool {
    req.headers()
        .get(http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .is_some_and(|t| allowed.contains(t.trim()))
}

fn deny(err: ConnectError) -> Response<ConnectRpcBody> {
    Response::builder()
        .status(err.http_status())
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(ConnectRpcBody::Full(Full::new(err.to_json())))
        .expect("static denial response always builds")
}

pin_project! {
    /// Future for [`TokenAuthService`]: pass through to the inner service, or
    /// return a pre-built denial. Generic over `F` so Send-ness is preserved.
    #[project = TokenAuthFutureProj]
    pub enum TokenAuthFuture<F> {
        Pass { #[pin] inner: F },
        Denied { response: Option<Response<ConnectRpcBody>> },
    }
}

impl<F> TokenAuthFuture<F> {
    fn pass(inner: F) -> Self {
        Self::Pass { inner }
    }
    fn denied(response: Response<ConnectRpcBody>) -> Self {
        Self::Denied { response: Some(response) }
    }
}

impl<F> std::future::Future for TokenAuthFuture<F>
where
    F: std::future::Future<Output = Result<Response<ConnectRpcBody>, Infallible>>,
{
    type Output = Result<Response<ConnectRpcBody>, Infallible>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        match self.project() {
            TokenAuthFutureProj::Pass { inner } => inner.poll(cx),
            TokenAuthFutureProj::Denied { response } => Poll::Ready(Ok(response
                .take()
                .expect("TokenAuthFuture::Denied polled after completion"))),
        }
    }
}
