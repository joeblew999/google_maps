//! Directions RPC: origin + destination -> route summaries (first leg).

use connectrpc::{ConnectError, Response, ServiceResult};
use google_maps::Client;
use google_maps::directions::request::location::Location;

use super::exec_await;
use crate::proto::maps::v1::{DirectionsResponse, OwnedDirectionsRequestView, Route};

pub(super) async fn directions(
    client: &Client,
    request: OwnedDirectionsRequestView,
) -> ServiceResult<DirectionsResponse> {
    let response = exec_await!(client
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
