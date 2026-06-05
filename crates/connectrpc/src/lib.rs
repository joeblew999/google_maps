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

// connect-rust's service trait uses `async fn`; our impls return `impl Future`
// which refines the trait's bound — the standard, expected pattern (see eliza/cf-do-locator).
#![allow(refining_impl_trait)]

/// Generated protobuf types + `MapsService` trait/ext + `MapsServiceClient`.
pub mod proto {
    connectrpc::include_generated!();
}

// The typed CLIENT + message types — available in every role, no `google_maps`
// dependency (so callers stay tiny — they bind over RPC instead of embedding).
pub use crate::proto::maps::v1::{
    DirectionsRequest, DirectionsResponse, DistanceMatrixElement, DistanceMatrixRequest,
    DistanceMatrixResponse, DistanceMatrixRow, ElevationRequest, ElevationResponse, ElevationResult,
    GeoResult, GeocodeRequest, GeocodeResponse, LatLng, MapsServiceClient, Place,
    PlacesAutocompleteRequest, PlacesAutocompleteResponse, PlacesNearbyRequest, PlacesNearbyResponse,
    Prediction, ReverseGeocodeRequest, ReverseGeocodeResponse, Route, TextSearchRequest,
    TextSearchResponse, TimeZoneRequest, TimeZoneResponse,
};

// The SERVER lives behind `_server` (enabled by `worker`/`native`) — it's the
// only part that pulls `google_maps`. It's split a file-per-feature under
// `server/` (mirroring upstream `src/`): `server/mod.rs` holds `MapsServer` + the
// thin `MapsService` trait impl that DELEGATES to one free function per feature
// (a Rust trait impl can't span files, so the dispatch stays central while the
// logic lives per feature: `geocoding.rs`, `directions.rs`, `distance_matrix.rs`,
// `elevation.rs`, `time_zone.rs`, `places.rs`).
#[cfg(feature = "_server")]
pub use crate::proto::maps::v1::MapsServiceExt;
#[cfg(feature = "_server")]
pub use server::MapsServer;
#[cfg(feature = "_server")]
mod server;

// Bearer-token auth layer for the shared server (gate consumers, protect the key).
#[cfg(feature = "_server")]
mod token_auth;
#[cfg(feature = "_server")]
pub use token_auth::TokenAuthLayer;
