//! Address Validation RPC: address lines + region -> verdict + standardized address.

use connectrpc::{ConnectError, Response, ServiceResult};
use google_maps::Client;
use google_maps::address_validation::PostalAddress;

use super::exec_await;
use crate::proto::maps::v1::{OwnedValidateAddressRequestView, ValidateAddressResponse};

pub(super) async fn validate_address(
    client: &Client,
    request: OwnedValidateAddressRequestView,
) -> ServiceResult<ValidateAddressResponse> {
    let region_code = if request.region_code.is_empty() {
        None
    } else {
        Some(request.region_code.to_string())
    };
    let address = PostalAddress {
        address_lines: request.address_lines.iter().map(|s| s.to_string()).collect(),
        region_code,
        ..Default::default()
    };

    let response = exec_await!(client.validate_address().address(address).build().execute())
        .map_err(|error| ConnectError::internal(error.to_string()))?;

    let result = &response.result;
    Ok(Response::new(ValidateAddressResponse {
        formatted_address: result.address.formatted_address.clone(),
        address_complete: result.verdict.address_complete,
        has_unconfirmed_components: result.verdict.has_unconfirmed_components,
        ..Default::default()
    }))
}
