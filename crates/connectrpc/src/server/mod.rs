//! The ConnectRPC server: `MapsServer` + a thin `MapsService` impl that
//! delegates to one free function per feature module.
//!
//! A Rust trait impl can't be spread across files, so the `impl MapsService`
//! block here stays as a central dispatch table; each method is a one-liner that
//! forwards to the matching feature module (mirroring upstream's `src/<feature>/`
//! layout). Add a feature = add a module + one delegating line below.

use connectrpc::{RequestContext, ServiceResult};
use google_maps::Client;

use crate::proto::maps::v1::{
    DirectionsResponse, DistanceMatrixResponse, ElevationResponse, GeocodeResponse, MapsService,
    NearestRoadsResponse, OwnedDirectionsRequestView, OwnedDistanceMatrixRequestView,
    OwnedElevationRequestView, OwnedGeocodeRequestView, OwnedNearestRoadsRequestView,
    OwnedPlaceDetailsRequestView, OwnedPlacePhotosRequestView, OwnedPlacesAutocompleteRequestView,
    OwnedPlacesNearbyRequestView, OwnedReverseGeocodeRequestView, OwnedSnapToRoadsRequestView,
    OwnedTextSearchRequestView, OwnedTimeZoneRequestView, OwnedValidateAddressRequestView,
    PlaceDetailsResponse, PlacePhotosResponse, PlacesAutocompleteResponse, PlacesNearbyResponse,
    ReverseGeocodeResponse, SnapToRoadsResponse, TextSearchResponse, TimeZoneResponse,
    ValidateAddressResponse,
};

mod address_validation;
mod directions;
mod distance_matrix;
mod elevation;
mod geocoding;
mod places;
mod roads;
mod time_zone;

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
        geocoding::geocode(&self.client, request).await
    }

    async fn reverse_geocode(
        &self,
        _ctx: RequestContext,
        request: OwnedReverseGeocodeRequestView,
    ) -> ServiceResult<ReverseGeocodeResponse> {
        geocoding::reverse_geocode(&self.client, request).await
    }

    async fn directions(
        &self,
        _ctx: RequestContext,
        request: OwnedDirectionsRequestView,
    ) -> ServiceResult<DirectionsResponse> {
        directions::directions(&self.client, request).await
    }

    async fn elevation(
        &self,
        _ctx: RequestContext,
        request: OwnedElevationRequestView,
    ) -> ServiceResult<ElevationResponse> {
        elevation::elevation(&self.client, request).await
    }

    async fn time_zone(
        &self,
        _ctx: RequestContext,
        request: OwnedTimeZoneRequestView,
    ) -> ServiceResult<TimeZoneResponse> {
        time_zone::time_zone(&self.client, request).await
    }

    async fn text_search(
        &self,
        _ctx: RequestContext,
        request: OwnedTextSearchRequestView,
    ) -> ServiceResult<TextSearchResponse> {
        places::text_search(&self.client, request).await
    }

    async fn distance_matrix(
        &self,
        _ctx: RequestContext,
        request: OwnedDistanceMatrixRequestView,
    ) -> ServiceResult<DistanceMatrixResponse> {
        distance_matrix::distance_matrix(&self.client, request).await
    }

    async fn places_autocomplete(
        &self,
        _ctx: RequestContext,
        request: OwnedPlacesAutocompleteRequestView,
    ) -> ServiceResult<PlacesAutocompleteResponse> {
        places::autocomplete(&self.client, request).await
    }

    async fn places_nearby(
        &self,
        _ctx: RequestContext,
        request: OwnedPlacesNearbyRequestView,
    ) -> ServiceResult<PlacesNearbyResponse> {
        places::nearby(&self.client, request).await
    }

    async fn place_details(
        &self,
        _ctx: RequestContext,
        request: OwnedPlaceDetailsRequestView,
    ) -> ServiceResult<PlaceDetailsResponse> {
        places::place_details(&self.client, request).await
    }

    async fn place_photos(
        &self,
        _ctx: RequestContext,
        request: OwnedPlacePhotosRequestView,
    ) -> ServiceResult<PlacePhotosResponse> {
        places::place_photos(&self.client, request).await
    }

    async fn snap_to_roads(
        &self,
        _ctx: RequestContext,
        request: OwnedSnapToRoadsRequestView,
    ) -> ServiceResult<SnapToRoadsResponse> {
        roads::snap_to_roads(&self.client, request).await
    }

    async fn nearest_roads(
        &self,
        _ctx: RequestContext,
        request: OwnedNearestRoadsRequestView,
    ) -> ServiceResult<NearestRoadsResponse> {
        roads::nearest_roads(&self.client, request).await
    }

    async fn validate_address(
        &self,
        _ctx: RequestContext,
        request: OwnedValidateAddressRequestView,
    ) -> ServiceResult<ValidateAddressResponse> {
        address_validation::validate_address(&self.client, request).await
    }
}

// The only per-target difference: Cloudflare's `worker::Fetch` futures are
// `!Send`, so they need `.into_send()` before `.await`; native (reqwest) futures
// are already `Send`. This macro keeps every feature's RPC body identical. It's
// fully-pathed (`::worker::send::IntoSendFuture`) so feature modules don't each
// import the trait. Defined AFTER the `mod` declarations (so it's not textually
// in their scope) and re-exported via `pub(crate) use` — each feature module
// pulls it explicitly with `use super::exec_await;`.
#[cfg(feature = "worker")]
macro_rules! exec_await {
    ($e:expr) => {{ ::worker::send::IntoSendFuture::into_send($e).await }};
}
#[cfg(not(feature = "worker"))]
macro_rules! exec_await {
    ($e:expr) => {{ $e.await }};
}
pub(crate) use exec_await;
