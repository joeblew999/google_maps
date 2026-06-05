//! Time Zone RPC: coordinates (+ optional unix timestamp) -> tz id/name/offsets.

use connectrpc::{ConnectError, Response, ServiceResult};
use google_maps::Client;
use google_maps::types::LatLng;

use super::exec_await;
use crate::proto::maps::v1::{OwnedTimeZoneRequestView, TimeZoneResponse};

pub(super) async fn time_zone(
    client: &Client,
    request: OwnedTimeZoneRequestView,
) -> ServiceResult<TimeZoneResponse> {
    let latlng = LatLng::try_from_f64(request.latitude, request.longitude)
        .map_err(|error| ConnectError::internal(error.to_string()))?;
    let secs = if request.timestamp_seconds > 0 { request.timestamp_seconds } else { 1_700_000_000 };
    let ts = chrono::DateTime::from_timestamp(secs, 0)
        .ok_or_else(|| ConnectError::internal("invalid timestamp"))?;
    let response = exec_await!(client.time_zone(latlng, ts).execute())
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
