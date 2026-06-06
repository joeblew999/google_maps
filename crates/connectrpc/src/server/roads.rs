//! Roads RPCs: snap a GPS path to the road network, and find the nearest road
//! for arbitrary points. (Speed Limits is not implemented in the base crate.)

use connectrpc::{ConnectError, Response, ServiceResult};
use google_maps::Client;
use google_maps::types::LatLng;
use rust_decimal::prelude::ToPrimitive;

use super::exec_await;
use crate::proto::maps::v1::{
    NearestRoadsResponse, OwnedNearestRoadsRequestView, OwnedSnapToRoadsRequestView,
    SnapToRoadsResponse, SnappedPoint,
};

/// Shared mapping from the crate's snapped point to the slim proto one.
fn to_pb(sp: &google_maps::roads::snapped_point::SnappedPoint) -> SnappedPoint {
    SnappedPoint {
        latitude: sp.location.lat.to_f64().unwrap_or_default(),
        longitude: sp.location.lng.to_f64().unwrap_or_default(),
        place_id: sp.place_id.clone().unwrap_or_default(),
        original_index: sp.origin_index.map(|i| i as u32).unwrap_or_default(),
        ..Default::default()
    }
}

fn path_from(points: impl Iterator<Item = (f64, f64)>) -> Result<Vec<LatLng>, ConnectError> {
    points
        .map(|(lat, lng)| LatLng::try_from_f64(lat, lng))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| ConnectError::internal(error.to_string()))
}

pub(super) async fn snap_to_roads(
    client: &Client,
    request: OwnedSnapToRoadsRequestView,
) -> ServiceResult<SnapToRoadsResponse> {
    let path = path_from(request.path.iter().map(|p| (p.latitude, p.longitude)))?;
    let response = exec_await!(client
        .snap_to_roads(path)
        .with_interpolation(request.interpolate)
        .execute())
    .map_err(|error| ConnectError::internal(error.to_string()))?;

    let snapped_points = response.snapped_points.iter().map(to_pb).collect();
    Ok(Response::new(SnapToRoadsResponse { snapped_points, ..Default::default() }))
}

pub(super) async fn nearest_roads(
    client: &Client,
    request: OwnedNearestRoadsRequestView,
) -> ServiceResult<NearestRoadsResponse> {
    let points = path_from(request.points.iter().map(|p| (p.latitude, p.longitude)))?;
    let response = exec_await!(client.nearest_roads(points).execute())
        .map_err(|error| ConnectError::internal(error.to_string()))?;

    let snapped_points = response.snapped_points.iter().map(to_pb).collect();
    Ok(Response::new(NearestRoadsResponse { snapped_points, ..Default::default() }))
}
