//! Elevation RPC: coordinates -> elevation (metres above sea level).

use connectrpc::{ConnectError, Response, ServiceResult};
use google_maps::Client;
use google_maps::types::LatLng;
use rust_decimal::prelude::ToPrimitive;

use super::exec_await;
use crate::proto::maps::v1::{ElevationResponse, ElevationResult, OwnedElevationRequestView};

pub(super) async fn elevation(
    client: &Client,
    request: OwnedElevationRequestView,
) -> ServiceResult<ElevationResponse> {
    let latlng = LatLng::try_from_f64(request.latitude, request.longitude)
        .map_err(|error| ConnectError::internal(error.to_string()))?;
    let response = exec_await!(client.elevation().for_positional_request(latlng).execute())
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
