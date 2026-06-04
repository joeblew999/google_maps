//! Example Cloudflare Worker exercising the `google_maps` crate's `worker`
//! transport — the wasm32 path that uses `worker::Fetch` instead of `reqwest`.
//!
//! Routes (all read the API key from the `GOOGLE_MAPS_API_KEY` secret):
//!
//! - `GET /geocode?address=...`              → forward geocoding
//! - `GET /reverse?lat=...&lng=...`          → reverse geocoding
//! - `GET /directions?origin=...&destination=...` → driving directions
//! - `GET /text-search?query=...`            → Places (New) text search
//!
//! Every handler uses the crate's *normal* `Client` + `.execute()` API — the
//! same code you would write for a native reqwest target. Only the build
//! features differ (see `Cargo.toml`).

use google_maps::Client;
use google_maps::directions::request::location::Location;
use google_maps::places_new::FieldMask;
use google_maps::types::LatLng;
use worker::{Env, Request, Response, Result, RouteContext, Router, event};

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    console_error_panic_hook::set_once();

    Router::new()
        .get("/", |_req, _ctx| {
            Response::ok(
                "google_maps worker transport demo\n\n\
                 GET /geocode?address=...\n\
                 GET /reverse?lat=...&lng=...\n\
                 GET /directions?origin=...&destination=...\n\
                 GET /text-search?query=...\n",
            )
        })
        .get_async("/geocode", geocode)
        .get_async("/reverse", reverse)
        .get_async("/directions", directions)
        .get_async("/text-search", text_search)
        .run(req, env)
        .await
}

/// Reads the Google Maps API key from the Worker secret binding. Never logged,
/// never echoed back to the caller.
fn client(ctx: &RouteContext<()>) -> Result<Client> {
    let key = ctx.secret("GOOGLE_MAPS_API_KEY")?.to_string();
    Ok(Client::new(key))
}

/// Returns the first value of query parameter `name`, or `None`.
fn param(req: &Request, name: &str) -> Option<String> {
    req.url()
        .ok()?
        .query_pairs()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.into_owned())
}

async fn geocode(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let Some(address) = param(&req, "address") else {
        return Response::error("missing ?address=", 400);
    };
    let client = client(&ctx)?;
    match client.geocoding().with_address(address).execute().await {
        Ok(response) => Response::from_json(&response),
        Err(error) => Response::error(format!("geocode failed: {error}"), 502),
    }
}

async fn reverse(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let (Some(lat), Some(lng)) = (param(&req, "lat"), param(&req, "lng")) else {
        return Response::error("missing ?lat= and ?lng=", 400);
    };
    let (Ok(lat), Ok(lng)) = (lat.parse::<f64>(), lng.parse::<f64>()) else {
        return Response::error("lat/lng must be numbers", 400);
    };
    let latlng = match LatLng::try_from_f64(lat, lng) {
        Ok(latlng) => latlng,
        Err(error) => return Response::error(format!("bad coordinates: {error}"), 400),
    };
    let client = client(&ctx)?;
    match client.reverse_geocoding(latlng).execute().await {
        Ok(response) => Response::from_json(&response),
        Err(error) => Response::error(format!("reverse geocode failed: {error}"), 502),
    }
}

async fn directions(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let (Some(origin), Some(destination)) = (param(&req, "origin"), param(&req, "destination"))
    else {
        return Response::error("missing ?origin= and ?destination=", 400);
    };
    let client = client(&ctx)?;
    match client
        .directions(Location::from_address(origin), Location::from_address(destination))
        .execute()
        .await
    {
        Ok(response) => Response::from_json(&response),
        Err(error) => Response::error(format!("directions failed: {error}"), 502),
    }
}

async fn text_search(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let Some(query) = param(&req, "query") else {
        return Response::error("missing ?query=", 400);
    };
    let client = client(&ctx)?;
    // `FieldMask::All` keeps the example simple; production callers should pass a
    // specific mask to control cost (Google bills by the fields returned).
    match client
        .text_search(query)
        .field_mask(FieldMask::All)
        .execute()
        .await
    {
        Ok(response) => Response::from_json(&response),
        Err(error) => Response::error(format!("text search failed: {error}"), 502),
    }
}
