//! Places (New) RPCs: text search, autocomplete, nearby search, place details,
//! and place photos. These share the slim `Place` mapping, so they live together
//! (mirroring upstream's `src/places_new/`).

use connectrpc::{ConnectError, Response, ServiceResult};
use google_maps::Client;
use google_maps::places_new::FieldMask;
use rust_decimal::prelude::ToPrimitive;

use super::exec_await;
use crate::proto::maps::v1::{
    OwnedPlaceDetailsRequestView, OwnedPlacePhotosRequestView, OwnedPlacesAutocompleteRequestView,
    OwnedPlacesNearbyRequestView, OwnedTextSearchRequestView, Place as PbPlace,
    PlaceDetailsResponse, PlacePhotosResponse, PlacesAutocompleteResponse, PlacesNearbyResponse,
    Prediction, TextSearchResponse,
};

pub(super) async fn text_search(
    client: &Client,
    request: OwnedTextSearchRequestView,
) -> ServiceResult<TextSearchResponse> {
    let response = exec_await!(client
        .text_search(request.query)
        .field_mask(FieldMask::All)
        .execute())
    .map_err(|error| ConnectError::internal(error.to_string()))?;

    let places = response.response().places().iter().map(place_to_pb).collect();

    Ok(Response::new(TextSearchResponse { places, ..Default::default() }))
}

pub(super) async fn autocomplete(
    client: &Client,
    request: OwnedPlacesAutocompleteRequestView,
) -> ServiceResult<PlacesAutocompleteResponse> {
    let response = exec_await!(client.autocomplete(request.input).execute())
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

    Ok(Response::new(PlacesAutocompleteResponse { predictions, ..Default::default() }))
}

pub(super) async fn nearby(
    client: &Client,
    request: OwnedPlacesNearbyRequestView,
) -> ServiceResult<PlacesNearbyResponse> {
    // (lat, lng, radius_metres) -> a circular location restriction.
    let builder = client
        .nearby_search((request.latitude, request.longitude, request.radius_meters))
        .map_err(|error| ConnectError::internal(error.to_string()))?;
    let response = exec_await!(builder.field_mask(FieldMask::All).execute())
        .map_err(|error| ConnectError::internal(error.to_string()))?;

    let places = response.iter().map(place_to_pb).collect();

    Ok(Response::new(PlacesNearbyResponse { places, ..Default::default() }))
}

pub(super) async fn place_details(
    client: &Client,
    request: OwnedPlaceDetailsRequestView,
) -> ServiceResult<PlaceDetailsResponse> {
    let builder = client
        .place_details(request.place_id)
        .map_err(|error| ConnectError::internal(error.to_string()))?;
    let response = exec_await!(builder.field_mask(FieldMask::All).execute())
        .map_err(|error| ConnectError::internal(error.to_string()))?;

    // `Response` derefs to `Place`, so `&response` coerces to `&Place`.
    Ok(Response::new(PlaceDetailsResponse {
        place: Some(place_to_pb(&response)).into(),
        ..Default::default()
    }))
}

pub(super) async fn place_photos(
    client: &Client,
    request: OwnedPlacePhotosRequestView,
) -> ServiceResult<PlacePhotosResponse> {
    let max_width = if request.max_width_px > 0 { request.max_width_px } else { 400 };
    let builder = client
        .place_photos_uri(request.photo_name)
        .map_err(|error| ConnectError::internal(error.to_string()))?;
    let response = exec_await!(builder.max_width_px(max_width).execute())
        .map_err(|error| ConnectError::internal(error.to_string()))?;

    Ok(Response::new(PlacePhotosResponse {
        photo_uri: response.uri.to_string(),
        ..Default::default()
    }))
}

/// Map a `google_maps` Places (New) `Place` to the slim proto `Place`. Shared by
/// Text Search, Nearby Search, and Place Details (phone/website/rating/photos are
/// only populated when the field mask + the endpoint return them).
fn place_to_pb(p: &google_maps::places_new::Place) -> PbPlace {
    PbPlace {
        display_name: p.display_name.as_ref().map(|d| d.text.clone()).unwrap_or_default(),
        formatted_address: p.formatted_address.clone().unwrap_or_default(),
        latitude: p.location.as_ref().map(|l| l.latitude.to_f64().unwrap_or_default()).unwrap_or_default(),
        longitude: p.location.as_ref().map(|l| l.longitude.to_f64().unwrap_or_default()).unwrap_or_default(),
        place_id: p.id.clone().unwrap_or_default(),
        phone_number: p.international_phone_number.clone().unwrap_or_default(),
        website_uri: p.website_uri.as_ref().map(|u| u.as_str().to_string()).unwrap_or_default(),
        rating: p.rating.unwrap_or_default(),
        photo_names: p.photos.iter().map(|ph| ph.name.clone()).collect(),
        ..Default::default()
    }
}
