//! Example: a Cloudflare worker that CALLS the shared maps Connect service.
//!
//! It binds over Connect RPC via `MapsServiceClient` + connect-rust's
//! `FetchTransport` — and does **not** depend on `google_maps`, so this worker's
//! wasm stays tiny. This is the composition story: many project workers call one
//! shared maps worker instead of each embedding the whole Maps client.
//!
//! `MAPS_URL` (var) points at the maps Connect worker (the shared one, or a
//! project worker that composes it). `MAPS_TOKEN` (secret) is this consumer's
//! Bearer token, sent on every call so the shared worker's token gate accepts it.
//! `GET /?address=Ottawa`.

use connectrpc::client::ClientConfig;
use connectrpc_workers::FetchTransport;
use google_maps_connectrpc::{GeocodeRequest, GeocodeResponse, MapsServiceClient};
use worker::{Context, Env, Request, Response, Result, event};

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let maps_url = env
        .var("MAPS_URL")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "http://localhost:8787/".to_string());

    let uri: http::Uri = match maps_url.parse() {
        Ok(u) => u,
        Err(e) => return Response::error(format!("bad MAPS_URL: {e}"), 500),
    };
    let transport = match FetchTransport::new(uri.clone()) {
        Ok(t) => t,
        Err(e) => return Response::error(format!("transport: {e}"), 500),
    };
    // Attach this consumer's Bearer token so the shared worker's gate accepts us.
    let token = env.var("MAPS_TOKEN").map(|v| v.to_string()).unwrap_or_default();
    let config = ClientConfig::new(uri).with_default_header("authorization", format!("Bearer {token}"));
    let client = MapsServiceClient::new(transport, config);

    let address = req
        .url()
        .ok()
        .and_then(|u| u.query_pairs().find(|(k, _)| k == "address").map(|(_, v)| v.into_owned()))
        .unwrap_or_else(|| "Ottawa".to_string());

    match client.geocode(GeocodeRequest { address, ..Default::default() }).await {
        Ok(unary) => { let resp: GeocodeResponse = unary.into_owned(); Response::from_json(&resp) },
        Err(e) => Response::error(format!("maps rpc failed: {e}"), 502),
    }
}
