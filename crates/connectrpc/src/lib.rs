//! Reusable ConnectRPC layer for Google Maps — one proto, three roles.
//!
//! - **Server** (`worker` / `native` feature): [`MapsServer`] embeds a
//!   `google_maps::Client` and answers RPCs. Mount it on a Connect router.
//! - **Client** (`client` feature, **no `google_maps`**): [`MapsServiceClient`]
//!   lets any worker/project *call* the maps service over Connect RPC, keeping
//!   its own deployment tiny (it binds over RPC instead of embedding the lib).
//!
//! Server (Cloudflare or native):
//! ```ignore
//! let router = Arc::new(MapsServer::new(client)).register(RpcRouter::new());
//! ```
//! Client (e.g. another Cloudflare worker, via connect-rust's worker transport):
//! ```ignore
//! use connectrpc::client::ClientConfig;
//! use connectrpc_workers::FetcherTransport;
//! use google_maps_connectrpc::MapsServiceClient;
//! let maps = MapsServiceClient::new(FetcherTransport::new(env.service("MAPS")?),
//!                                   ClientConfig::new("http://maps/".parse()?));
//! let res = maps.geocode(GeocodeRequest { address: "Ottawa".into(), ..Default::default() }).await?;
//! ```

/// Generated protobuf types + `MapsService` trait/ext + `MapsServiceClient`.
pub mod proto {
    connectrpc::include_generated!();
}

// The typed CLIENT + message types — available in every role, no `google_maps`
// dependency (so callers stay tiny — they bind over RPC instead of embedding).
pub use crate::proto::maps::v1::{
    GeocodeRequest, GeocodeResponse, MapsServiceClient, TextSearchRequest, TextSearchResponse,
};

// The SERVER lives behind `_server` (enabled by `worker`/`native`) — it's the
// only part that pulls `google_maps`.
#[cfg(feature = "_server")]
pub use crate::proto::maps::v1::MapsServiceExt;
#[cfg(feature = "_server")]
pub use server::MapsServer;

#[cfg(feature = "_server")]
mod server {
    use connectrpc::{ConnectError, RequestContext, Response, ServiceResult};
    use google_maps::Client;
    use google_maps::places_new::FieldMask;
    use rust_decimal::prelude::ToPrimitive;

    #[cfg(feature = "worker")]
    use worker::send::IntoSendFuture;

    use crate::proto::maps::v1::{
        GeoResult, GeocodeResponse, MapsService, OwnedGeocodeRequestView,
        OwnedTextSearchRequestView, Place as PbPlace, TextSearchResponse,
    };

    // The only per-target difference: Cloudflare's `worker::Fetch` futures are
    // `!Send`, so they need `.into_send()` before `.await`; native (reqwest)
    // futures are already `Send`. This macro keeps the RPC bodies identical.
    #[cfg(feature = "worker")]
    macro_rules! exec_await {
        ($e:expr) => {{ $e.into_send().await }};
    }
    #[cfg(not(feature = "worker"))]
    macro_rules! exec_await {
        ($e:expr) => {{ $e.await }};
    }

    /// ConnectRPC service backed by a `google_maps::Client`.
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
}
