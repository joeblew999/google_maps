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

/// Permissive CORS so a browser web client (a different origin) can call the
/// Connect RPCs. We carry the token in the `Authorization` header (not cookies),
/// so wildcard origin is safe — but note `*` does NOT cover `Authorization`, so
/// it must be listed explicitly in allow-headers.
fn apply_cors(headers: &mut http::HeaderMap) {
    let set = |h: &mut http::HeaderMap, k: &'static str, v: &'static str| {
        if let Ok(val) = http::HeaderValue::from_str(v) {
            h.insert(k, val);
        }
    };
    set(headers, "access-control-allow-origin", "*");
    set(headers, "access-control-allow-methods", "POST, GET, OPTIONS");
    set(
        headers,
        "access-control-allow-headers",
        "authorization, content-type, connect-protocol-version, connect-timeout-ms, x-user-agent, x-grpc-web",
    );
    set(headers, "access-control-max-age", "86400");
}

#[event(fetch, respond_with_errors)]
async fn fetch(
    req: HttpRequest,
    env: Env,
    _ctx: Context,
) -> worker::Result<http::Response<ConnectRpcBody>> {
    // CORS preflight: answer BEFORE the token gate (preflight never carries the
    // Authorization header by design, so gating it would break every browser call).
    if req.method() == http::Method::OPTIONS {
        // 200 (not 204): the workers runtime rejects a body on a 204, and our
        // ConnectRpcBody always carries a (here empty) body. Browsers accept any 2xx.
        let mut resp = http::Response::builder()
            .status(200)
            .body(ConnectRpcBody::Full(Full::new(bytes::Bytes::new())))
            .map_err(|e| worker::Error::RustError(format!("preflight: {e}")))?;
        apply_cors(resp.headers_mut());
        return Ok(resp);
    }

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

    let mut resp = svc
        .call(req)
        .await
        .map_err(|e| worker::Error::RustError(format!("rpc dispatch: {e}")))?;
    // CORS on the actual response too, so the browser accepts the RPC result
    // (and the gate's 401 — otherwise the browser hides the real status).
    apply_cors(resp.headers_mut());
    Ok(resp)
}
