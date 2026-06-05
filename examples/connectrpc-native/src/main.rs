//! Native (axum) server exposing the SAME `maps.v1.MapsService` as the Cloudflare
//! worker example — proving one proto + one `MapsServer` serves both runtimes.
//! Only the build features differ (`native` here vs `worker` there).
//!
//! Run: `GOOGLE_MAPS_API_KEY=... cargo run` (listens on 127.0.0.1:8080).
//! Call: `curl -X POST localhost:8080/maps.v1.MapsService/Geocode \
//!   -H 'Content-Type: application/json' -d '{"address":"Ottawa"}'`

use std::sync::Arc;

use connectrpc::Router as RpcRouter;
use google_maps_connectrpc::{MapsServer, MapsServiceExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let key = std::env::var("GOOGLE_MAPS_API_KEY").unwrap_or_default();
    let client = google_maps::Client::try_new(key)?;

    // Identical to the worker: build the service, register it on a Connect router.
    let router = Arc::new(MapsServer::new(client)).register(RpcRouter::new());

    // Native difference: serve the Connect router via axum instead of worker fetch.
    let app = axum::Router::new()
        .route("/health", axum::routing::get(|| async { "ok" }))
        .fallback_service(router.into_axum_service());

    let addr = std::env::var("ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("google-maps-connectrpc (native) listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
