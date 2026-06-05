//! Example Cloudflare Worker that mounts the reusable `google-maps-connectrpc`
//! service. Exposes typed Connect RPCs (`maps.v1.MapsService/Geocode`,
//! `/TextSearch`) backed by the `google_maps` worker transport.
//!
//! A TS/React client generated from `crates/connectrpc/proto/maps/v1/maps.proto`
//! can call these with full type safety. The API key comes from the
//! `GOOGLE_MAPS_API_KEY` secret.

use std::sync::Arc;

use connectrpc::{ConnectRpcBody, ConnectRpcService, Router as RpcRouter};
use google_maps_connectrpc::{MapsServer, MapsServiceExt, TokenAuthLayer};
use http_body_util::Full;
use tower::{Layer, Service};
use worker::{Context, Env, HttpRequest, event};

#[event(fetch, respond_with_errors)]
async fn fetch(
    req: HttpRequest,
    env: Env,
    _ctx: Context,
) -> worker::Result<http::Response<ConnectRpcBody>> {
    // Friendly landing for browser visitors (RPC paths fall through below).
    if req.uri().path() == "/" {
        let body = "google-maps-connectrpc demo\n\n\
            POST /maps.v1.MapsService/Geocode      {\"address\":\"Ottawa\"}\n\
            POST /maps.v1.MapsService/TextSearch   {\"query\":\"pizza in Ottawa\"}\n\n\
            Use a generated Connect client, or curl with Content-Type: application/json.\n";
        return http::Response::builder()
            .status(200)
            .header(http::header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .body(ConnectRpcBody::Full(Full::new(bytes::Bytes::from(body))))
            .map_err(|e| worker::Error::RustError(format!("landing: {e}")));
    }

    let client = google_maps::Client::new(env.secret("GOOGLE_MAPS_API_KEY")?.to_string());

    let router = Arc::new(MapsServer::new(client)).register(RpcRouter::new());

    // Gate consumers with a Bearer token (allow-list from the MAPS_TOKENS secret).
    // Empty/unset = deny all — the safe default for a worker holding a real key.
    let tokens = env.var("MAPS_TOKENS").map(|v| v.to_string()).unwrap_or_default();
    let mut svc = TokenAuthLayer::from_list(&tokens).layer(ConnectRpcService::new(router));

    svc.call(req)
        .await
        .map_err(|e| worker::Error::RustError(format!("rpc dispatch: {e}")))
}
