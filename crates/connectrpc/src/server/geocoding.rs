//! Geocoding RPCs: address -> coordinates, and coordinates -> addresses.

use connectrpc::{ConnectError, Response, ServiceResult};
use google_maps::Client;
use google_maps::types::LatLng;
use rust_decimal::prelude::ToPrimitive;

use super::exec_await;
use crate::proto::maps::v1::{
    GeoResult, GeocodeResponse, OwnedGeocodeRequestView, OwnedReverseGeocodeRequestView,
    ReverseGeocodeResponse,
};

pub(super) async fn geocode(
    client: &Client,
    request: OwnedGeocodeRequestView,
) -> ServiceResult<GeocodeResponse> {
    let response = exec_await!(client.geocoding().with_address(request.address).execute())
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

pub(super) async fn reverse_geocode(
    client: &Client,
    request: OwnedReverseGeocodeRequestView,
) -> ServiceResult<ReverseGeocodeResponse> {
    let latlng = LatLng::try_from_f64(request.latitude, request.longitude)
        .map_err(|error| ConnectError::internal(error.to_string()))?;
    let response = exec_await!(client.reverse_geocoding(latlng).execute())
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
