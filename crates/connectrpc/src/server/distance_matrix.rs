//! Distance Matrix RPC: N origins x M destinations -> per-pair distance/duration.
//! Inputs are coordinates (clients geocode addresses first — `Waypoint` only
//! converts from `LatLng`).

use connectrpc::{ConnectError, Response, ServiceResult};
use google_maps::Client;
use google_maps::types::LatLng;

use super::exec_await;
use crate::proto::maps::v1::{
    DistanceMatrixElement, DistanceMatrixResponse, DistanceMatrixRow, OwnedDistanceMatrixRequestView,
};

pub(super) async fn distance_matrix(
    client: &Client,
    request: OwnedDistanceMatrixRequestView,
) -> ServiceResult<DistanceMatrixResponse> {
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

    let response = exec_await!(client.distance_matrix(origins, destinations).execute())
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
