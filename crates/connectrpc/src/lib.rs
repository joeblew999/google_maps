//! Reusable ConnectRPC facade over the `google_maps` Cloudflare Workers transport.
//!
//! Link this crate from any `workers-rs` project and mount [`MapsServer`] on a
//! ConnectRPC router to expose Google Maps over typed Connect RPCs (great for a
//! generated TypeScript/React client). The slim proto lives in `proto/maps/v1`.
//!
//! ```ignore
//! use std::sync::Arc;
//! use connectrpc::{ConnectRpcService, Router as RpcRouter};
//! use google_maps_connectrpc::{MapsServer, proto::maps::v1::MapsServiceExt};
//! use tower::Service;
//!
//! let client = google_maps::Client::new(api_key);
//! let router = Arc::new(MapsServer::new(client)).register(RpcRouter::new());
//! let mut svc = ConnectRpcService::new(router);
//! let resp = svc.call(req).await?;
//! ```

use connectrpc::{ConnectError, RequestContext, Response, ServiceResult};
use google_maps::Client;
use google_maps::places_new::FieldMask;
use rust_decimal::prelude::ToPrimitive;

#[cfg(feature = "worker")]
use worker::send::IntoSendFuture;

// The only per-target difference: on Cloudflare the `worker::Fetch` futures are
// `!Send`, so they need `.into_send()` before `.await`; native (reqwest) futures
// are already `Send`. This macro hides that so the RPC bodies are identical.
#[cfg(feature = "worker")]
macro_rules! exec_await {
    ($e:expr) => {{ $e.into_send().await }};
}
#[cfg(not(feature = "worker"))]
macro_rules! exec_await {
    ($e:expr) => {{ $e.await }};
}

/// Generated protobuf types + the `MapsService` trait/ext (from `build.rs`).
pub mod proto {
    connectrpc::include_generated!();
}

use crate::proto::maps::v1::{
    GeoResult, GeocodeResponse, MapsService, OwnedGeocodeRequestView,
    OwnedTextSearchRequestView, Place as PbPlace, TextSearchResponse,
};

// Re-export so consumers can `.register(router)` without naming the proto path.
pub use crate::proto::maps::v1::MapsServiceExt;

/// ConnectRPC service backed by a `google_maps::Client` (worker transport).
pub struct MapsServer {
    client: Client,
}

impl MapsServer {
    #[must_use]
    pub const fn new(client: Client) -> Self {
        Self { client }
    }
}

impl MapsService for MapsServer {
    async fn geocode(
        &self,
        _ctx: RequestContext,
        request: OwnedGeocodeRequestView,
    ) -> ServiceResult<GeocodeResponse> {
        let response = exec_await!(self
            .client
            .geocoding()
            .with_address(request.address)
            .execute())
        .map_err(|error| ConnectError::internal(error.to_string()))?;

        let results = response
            .results
            .iter()
            .map(|r| GeoResult {
                formatted_address: r.formatted_address.clone(),
                latitude: r.geometry.location.lat.to_f64().unwrap_or_default(),
                longitude: r.geometry.location.lng.to_f64().unwrap_or_default(),
                ..Default::default()
            })
            .collect();

        Ok(Response::new(GeocodeResponse {
            results,
            status: format!("{:?}", response.status),
            ..Default::default()
        }))
    }

    async fn text_search(
        &self,
        _ctx: RequestContext,
        request: OwnedTextSearchRequestView,
    ) -> ServiceResult<TextSearchResponse> {
        let response = exec_await!(self
            .client
            .text_search(request.query)
            .field_mask(FieldMask::All)
            .execute())
        .map_err(|error| ConnectError::internal(error.to_string()))?;

        let places = response
            .response()
            .places()
            .iter()
            .map(|p| PbPlace {
                display_name: p.display_name.as_ref().map(|d| d.text.clone()).unwrap_or_default(),
                formatted_address: p.formatted_address.clone().unwrap_or_default(),
                ..Default::default()
            })
            .collect();

        Ok(Response::new(TextSearchResponse {
            places,
            ..Default::default()
        }))
    }
}
