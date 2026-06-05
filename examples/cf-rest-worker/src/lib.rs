//! Example Cloudflare Worker exercising the `google_maps` crate's `worker`
//! transport — the wasm32 path that uses `worker::Fetch` instead of `reqwest`.
//!
//! There's a route per API so each can be smoke-tested on Cloudflare. Every
//! handler uses the crate's *normal* `Client` + `.execute()` API — the same code
//! you'd write natively; only the build features differ (see `Cargo.toml`).
//!
//! All routes read the API key from the `GOOGLE_MAPS_API_KEY` secret. Maps API
//! *calls* require an OPEN billing account; without one you'll see Google's
//! `REQUEST_DENIED` / billing errors — which still prove the transport round-trips.
//!
//! Transport coverage: GET (geocode, reverse, directions, distance-matrix,
//! elevation, time-zone, roads, place-details), POST (text-search, autocomplete,
//! nearby-search, validate-address), binary GET (place photos — wired via
//! `get_binary_request`, exercised once you have a photo reference).

use google_maps::Client;
use google_maps::address_validation::PostalAddress;
use google_maps::directions::request::location::Location;
use google_maps::directions::request::waypoint::Waypoint;
use google_maps::places_new::FieldMask;
use google_maps::prelude::{DateTime, Utc};
use google_maps::types::LatLng;
use google_maps::prelude::Country;
use worker::{Env, Request, Response, Result, RouteContext, Router, event};

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    console_error_panic_hook::set_once();

    Router::new()
        .get("/", |_req, _ctx| {
            Response::ok(
                "google_maps worker transport demo — a route per API:\n\n\
                 GET  /geocode?address=...\n\
                 GET  /reverse?lat=..&lng=..\n\
                 GET  /directions?origin=..&destination=..\n\
                 GET  /distance-matrix\n\
                 GET  /elevation\n\
                 GET  /time-zone\n\
                 GET  /snap-to-roads\n\
                 GET  /nearest-roads\n\
                 GET  /text-search?query=...\n\
                 GET  /autocomplete?input=...\n\
                 GET  /nearby-search\n\
                 GET  /place-details?place_id=...\n\
                 GET  /validate-address\n",
            )
        })
        .get_async("/geocode", geocode)
        .get_async("/reverse", reverse)
        .get_async("/directions", directions)
        .get_async("/distance-matrix", distance_matrix)
        .get_async("/elevation", elevation)
        .get_async("/time-zone", time_zone)
        .get_async("/snap-to-roads", snap_to_roads)
        .get_async("/nearest-roads", nearest_roads)
        .get_async("/text-search", text_search)
        .get_async("/autocomplete", autocomplete)
        .get_async("/nearby-search", nearby_search)
        .get_async("/place-details", place_details)
        .get_async("/validate-address", validate_address)
        .run(req, env)
        .await
}

/// Reads the API key from the Worker secret binding. Never logged or echoed.
fn client(ctx: &RouteContext<()>) -> Result<Client> {
    Ok(Client::new(ctx.secret("GOOGLE_MAPS_API_KEY")?.to_string()))
}

fn param(req: &Request, name: &str) -> Option<String> {
    req.url().ok()?.query_pairs().find(|(k, _)| k == name).map(|(_, v)| v.into_owned())
}

/// Render any Debug response as a 200, or any crate error as a 502 — enough to
/// confirm the transport round-tripped.
fn done<T: std::fmt::Debug>(label: &str, r: std::result::Result<T, google_maps::Error>) -> Result<Response> {
    match r {
        Ok(v) => Response::ok(format!("{v:#?}")),
        Err(e) => Response::error(format!("{label} failed: {e}"), 502),
    }
}

// ---- GET APIs (worker get_request) -------------------------------------------

async fn geocode(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let Some(address) = param(&req, "address") else { return Response::error("missing ?address=", 400) };
    done("geocode", client(&ctx)?.geocoding().with_address(address).execute().await)
}

async fn reverse(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let (Some(lat), Some(lng)) = (param(&req, "lat"), param(&req, "lng")) else { return Response::error("missing ?lat=&lng=", 400) };
    let (Ok(lat), Ok(lng)) = (lat.parse::<f64>(), lng.parse::<f64>()) else { return Response::error("lat/lng must be numbers", 400) };
    let Ok(latlng) = LatLng::try_from_f64(lat, lng) else { return Response::error("bad coordinates", 400) };
    done("reverse geocode", client(&ctx)?.reverse_geocoding(latlng).execute().await)
}

async fn directions(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let (Some(o), Some(d)) = (param(&req, "origin"), param(&req, "destination")) else { return Response::error("missing ?origin=&destination=", 400) };
    done("directions", client(&ctx)?.directions(Location::from_address(o), Location::from_address(d)).execute().await)
}

async fn distance_matrix(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let origins = vec![Waypoint::from_address("One Microsoft Way, Redmond, WA")];
    let destinations = vec![Waypoint::from_address("101 Townsend St, San Francisco, CA")];
    done("distance-matrix", client(&ctx)?.distance_matrix(origins, destinations).execute().await)
}

async fn elevation(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let Ok(latlng) = LatLng::try_from_f64(39.739_154, -104.984_703) else { return Response::error("bad coordinates", 400) };
    done("elevation", client(&ctx)?.elevation().for_positional_request(latlng).execute().await)
}

async fn time_zone(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let Ok(latlng) = LatLng::try_from_f64(50.090_903, 14.400_512) else { return Response::error("bad coordinates", 400) };
    // Fixed timestamp (avoids chrono's wasm clock); 2023-11-14.
    let Some(ts) = DateTime::<Utc>::from_timestamp(1_700_000_000, 0) else { return Response::error("bad timestamp", 500) };
    done("time-zone", client(&ctx)?.time_zone(latlng, ts).execute().await)
}

async fn snap_to_roads(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let (Ok(a), Ok(b)) = (LatLng::try_from_f64(-35.278_01, 149.129_58), LatLng::try_from_f64(-35.280_32, 149.129_07)) else { return Response::error("bad coordinates", 400) };
    done("snap-to-roads", client(&ctx)?.snap_to_roads(vec![a, b]).execute().await)
}

async fn nearest_roads(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let Ok(p) = LatLng::try_from_f64(60.170_880, 24.942_795) else { return Response::error("bad coordinates", 400) };
    done("nearest-roads", client(&ctx)?.nearest_roads(vec![p]).execute().await)
}

async fn place_details(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let place_id = param(&req, "place_id").unwrap_or_else(|| "ChIJyZXV1jsioFMRC8PGIBAJbKA".to_string());
    let client = client(&ctx)?;
    let builder = match client.place_details(place_id) {
        Ok(b) => b,
        Err(e) => return Response::error(format!("place-details build failed: {e}"), 400),
    };
    done("place-details", builder.field_mask(FieldMask::All).execute().await)
}

// ---- POST APIs (worker post_request) -----------------------------------------

async fn text_search(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let Some(query) = param(&req, "query") else { return Response::error("missing ?query=", 400) };
    done("text-search", client(&ctx)?.text_search(query).field_mask(FieldMask::All).execute().await)
}

async fn autocomplete(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let input = param(&req, "input").unwrap_or_else(|| "pizza".to_string());
    done("autocomplete", client(&ctx)?.autocomplete(input).execute().await)
}

async fn nearby_search(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let client = client(&ctx)?;
    let builder = match client.nearby_search((53.536_66, -113.507_95, 5_000.0)) {
        Ok(b) => b,
        Err(e) => return Response::error(format!("nearby-search build failed: {e}"), 400),
    };
    done("nearby-search", builder.field_mask(FieldMask::All).execute().await)
}

async fn validate_address(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let address = PostalAddress::builder()
        .region_code(&Country::UnitedStates)
        .address_lines(vec!["1600 Amphitheatre Pkwy", "Mountain View, CA, 94043"])
        .build();
    done("validate-address", client(&ctx)?.validate_address().address(address).build().execute().await)
}
