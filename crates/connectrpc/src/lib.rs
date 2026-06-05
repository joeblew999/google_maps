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
// only part that pulls `google_maps`.
#[cfg(feature = "_server")]
pub use crate::proto::maps::v1::MapsServiceExt;
#[cfg(feature = "_server")]
pub use server::MapsServer;

// Bearer-token auth layer for the shared server (gate consumers, protect the key).
#[cfg(feature = "_server")]
mod token_auth;
#[cfg(feature = "_server")]
pub use token_auth::TokenAuthLayer;

#[cfg(feature = "_server")]
mod server {
    use connectrpc::{ConnectError, RequestContext, Response, ServiceResult};
    use google_maps::Client;
    use google_maps::directions::request::location::Location;
    use google_maps::places_new::FieldMask;
    use google_maps::types::LatLng;
    use rust_decimal::prelude::ToPrimitive;

    #[cfg(feature = "worker")]
    use worker::send::IntoSendFuture;

    use crate::proto::maps::v1::{
        DirectionsResponse, DistanceMatrixElement, DistanceMatrixResponse, DistanceMatrixRow,
        ElevationResponse, ElevationResult, GeoResult, GeocodeResponse, MapsService,
        OwnedDirectionsRequestView, OwnedDistanceMatrixRequestView, OwnedElevationRequestView,
        OwnedGeocodeRequestView, OwnedPlacesAutocompleteRequestView, OwnedPlacesNearbyRequestView,
        OwnedReverseGeocodeRequestView, OwnedTextSearchRequestView, OwnedTimeZoneRequestView,
        Place as PbPlace, PlacesAutocompleteResponse, PlacesNearbyResponse, Prediction,
        ReverseGeocodeResponse, Route, TextSearchResponse, TimeZoneResponse,
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

        async fn reverse_geocode(
            &self,
            _ctx: RequestContext,
            request: OwnedReverseGeocodeRequestView,
        ) -> ServiceResult<ReverseGeocodeResponse> {
            let latlng = LatLng::try_from_f64(request.latitude, request.longitude)
                .map_err(|error| ConnectError::internal(error.to_string()))?;
            let response = exec_await!(self.client.reverse_geocoding(latlng).execute())
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

            Ok(Response::new(ReverseGeocodeResponse {
                results,
                status: format!("{:?}", response.status),
                ..Default::default()
            }))
        }

        async fn directions(
            &self,
            _ctx: RequestContext,
            request: OwnedDirectionsRequestView,
        ) -> ServiceResult<DirectionsResponse> {
            let response = exec_await!(self
                .client
                .directions(
                    Location::from_address(request.origin),
                    Location::from_address(request.destination),
                )
                .execute())
            .map_err(|error| ConnectError::internal(error.to_string()))?;

            let routes = response
                .routes
                .iter()
                .map(|r| {
                    let leg = r.legs.first();
                    Route {
                        summary: r.summary.clone(),
                        distance: leg.map(|l| l.distance.text.clone()).unwrap_or_default(),
                        duration: leg.map(|l| l.duration.text.clone()).unwrap_or_default(),
                        start_address: leg.map(|l| l.start_address.clone()).unwrap_or_default(),
                        end_address: leg.map(|l| l.end_address.clone()).unwrap_or_default(),
                        distance_meters: leg.map(|l| l.distance.value).unwrap_or_default(),
                        duration_seconds: leg.map(|l| l.duration.value.num_seconds()).unwrap_or_default(),
                        ..Default::default()
                    }
                })
                .collect();

            Ok(Response::new(DirectionsResponse {
                routes,
                status: format!("{:?}", response.status),
                ..Default::default()
            }))
        }

        async fn elevation(
            &self,
            _ctx: RequestContext,
            request: OwnedElevationRequestView,
        ) -> ServiceResult<ElevationResponse> {
            let latlng = LatLng::try_from_f64(request.latitude, request.longitude)
                .map_err(|error| ConnectError::internal(error.to_string()))?;
            let response = exec_await!(self.client.elevation().for_positional_request(latlng).execute())
                .map_err(|error| ConnectError::internal(error.to_string()))?;

            let results = response
                .results
                .iter()
                .map(|p| ElevationResult {
                    elevation: p.elevation,
                    latitude: p.location.lat.to_f64().unwrap_or_default(),
                    longitude: p.location.lng.to_f64().unwrap_or_default(),
                    resolution: p.resolution.unwrap_or_default(),
                    ..Default::default()
                })
                .collect();

            Ok(Response::new(ElevationResponse { results, ..Default::default() }))
        }

        async fn time_zone(
            &self,
            _ctx: RequestContext,
            request: OwnedTimeZoneRequestView,
        ) -> ServiceResult<TimeZoneResponse> {
            let latlng = LatLng::try_from_f64(request.latitude, request.longitude)
                .map_err(|error| ConnectError::internal(error.to_string()))?;
            let secs = if request.timestamp_seconds > 0 { request.timestamp_seconds } else { 1_700_000_000 };
            let ts = chrono::DateTime::from_timestamp(secs, 0)
                .ok_or_else(|| ConnectError::internal("invalid timestamp"))?;
            let response = exec_await!(self.client.time_zone(latlng, ts).execute())
                .map_err(|error| ConnectError::internal(error.to_string()))?;

            Ok(Response::new(TimeZoneResponse {
                time_zone_id: response.time_zone_id.map(|tz| tz.name().to_string()).unwrap_or_default(),
                time_zone_name: response.time_zone_name.clone().unwrap_or_default(),
                raw_offset_seconds: response.raw_offset.unwrap_or_default(),
                dst_offset_seconds: response.dst_offset.unwrap_or_default(),
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
                .map(place_to_pb)
                .collect();

            Ok(Response::new(TextSearchResponse {
                places,
                ..Default::default()
            }))
        }

        async fn distance_matrix(
            &self,
            _ctx: RequestContext,
            request: OwnedDistanceMatrixRequestView,
        ) -> ServiceResult<DistanceMatrixResponse> {
            // Origins/destinations arrive as coordinates (clients geocode first).
            let origins = request
                .origins
                .iter()
                .map(|p| LatLng::try_from_f64(p.latitude, p.longitude))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| ConnectError::internal(error.to_string()))?;
            let destinations = request
                .destinations
                .iter()
                .map(|p| LatLng::try_from_f64(p.latitude, p.longitude))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| ConnectError::internal(error.to_string()))?;

            let response = exec_await!(self
                .client
                .distance_matrix(origins, destinations)
                .execute())
            .map_err(|error| ConnectError::internal(error.to_string()))?;

            let rows = response
                .rows
                .iter()
                .map(|row| DistanceMatrixRow {
                    elements: row
                        .elements
                        .iter()
                        .map(|el| DistanceMatrixElement {
                            distance_meters: el.distance.as_ref().map(|d| d.value).unwrap_or_default(),
                            duration_seconds: el
                                .duration
                                .as_ref()
                                .map(|d| d.value.num_seconds())
                                .unwrap_or_default(),
                            status: format!("{:?}", el.status),
                            ..Default::default()
                        })
                        .collect(),
                    ..Default::default()
                })
                .collect();

            Ok(Response::new(DistanceMatrixResponse {
                rows,
                status: format!("{:?}", response.status),
                ..Default::default()
            }))
        }

        async fn places_autocomplete(
            &self,
            _ctx: RequestContext,
            request: OwnedPlacesAutocompleteRequestView,
        ) -> ServiceResult<PlacesAutocompleteResponse> {
            let response = exec_await!(self.client.autocomplete(request.input).execute())
                .map_err(|error| ConnectError::internal(error.to_string()))?;

            let predictions = response
                .response()
                .places()
                .map(|p| Prediction {
                    description: p.text.text.clone(),
                    place_id: p.place_id.clone(),
                    ..Default::default()
                })
                .collect();

            Ok(Response::new(PlacesAutocompleteResponse {
                predictions,
                ..Default::default()
            }))
        }

        async fn places_nearby(
            &self,
            _ctx: RequestContext,
            request: OwnedPlacesNearbyRequestView,
        ) -> ServiceResult<PlacesNearbyResponse> {
            // (lat, lng, radius_metres) -> a circular location restriction.
            let builder = self
                .client
                .nearby_search((request.latitude, request.longitude, request.radius_meters))
                .map_err(|error| ConnectError::internal(error.to_string()))?;
            let response = exec_await!(builder.field_mask(FieldMask::All).execute())
                .map_err(|error| ConnectError::internal(error.to_string()))?;

            let places = response.iter().map(place_to_pb).collect();

            Ok(Response::new(PlacesNearbyResponse {
                places,
                ..Default::default()
            }))
        }
    }

    /// Map a `google_maps` Places (New) `Place` to the slim proto `Place`
    /// (shared by Text Search and Nearby Search).
    fn place_to_pb(p: &google_maps::places_new::Place) -> PbPlace {
        PbPlace {
            display_name: p.display_name.as_ref().map(|d| d.text.clone()).unwrap_or_default(),
            formatted_address: p.formatted_address.clone().unwrap_or_default(),
            latitude: p.location.as_ref().map(|l| l.latitude.to_f64().unwrap_or_default()).unwrap_or_default(),
            longitude: p.location.as_ref().map(|l| l.longitude.to_f64().unwrap_or_default()).unwrap_or_default(),
            place_id: p.id.clone().unwrap_or_default(),
            ..Default::default()
        }
    }
}
