# Upstream issue for leontoeides/google_maps

> ✅ FILED 2026-06-04: https://github.com/leontoeides/google_maps/issues/44
> (Original draft retained below for reference.)

---

**Title:** Runs on Cloudflare Workers (wasm32) via an optional `worker` transport — happy to upstream

**Body:**

Hi, and thanks for this crate — the trait-based request/transport split made the following much
easier than I expected.

I've got `google_maps` running **inside a Cloudflare Worker** (`wasm32-unknown-unknown`,
`workers-rs`) and would like to offer it back upstream. Everything is additive and feature-gated —
**the default `reqwest` build is unchanged** — so I don't think it affects existing users.

Work-in-progress fork: https://github.com/joeblew999/google_maps

## 1. Bug fix (independent of the Workers stuff)

`--no-default-features` builds currently fail to compile. In `src/error.rs` the `HttpWithBody`
variant is ungated, but its field type `HttpErrorStatus` *and* its `classify()` match arm are both
`#[cfg(feature = "reqwest")]`. Without `reqwest` that yields E0412/E0425 (missing type) and E0004
(non-exhaustive match). One-line fix — gate the variant to match:

```rust
#[cfg(feature = "reqwest")]   // <-- add this
#[error("HTTP error {status}: {body:?}")]
#[diagnostic(code(google_maps::http_with_body))]
HttpWithBody { status: HttpErrorStatus, body: String },
```

I'm happy to send this as a standalone PR regardless of the rest.

## 2. Optional Cloudflare Workers transport

The idea: a `worker` feature that is *mutually alternative* to `reqwest` and supplies the central
send methods via [`worker::Fetch`](https://crates.io/crates/worker) instead of `reqwest`. Because
`query_string()` / `query_url()` are already transport-agnostic and each `execute()` just delegates
to `Client::get_request()` / `post_request()`, the change is small and localised:

- `worker` feature = `["dep:worker", "dep:http"]`; `reqwest` also gains `dep:http`.
- `RequestHeaders::request_headers()` returns `http::HeaderMap` instead of
  `reqwest::header::HeaderMap` (they're the same re-exported type, so the reqwest path is unchanged).
- New `cfg(all(worker, not(reqwest)))` modules implementing `get_request`/`post_request` with the
  **same signatures** — so no per-API `execute()` code changes. No rate limiter / `backon` retry on
  this path (Workers have no timer).
- New worker-gated `Error::Worker` / `Error::WorkerHttp` variants + `classify()` arms.
- A handful of `cfg(feature = "reqwest")` gates widened to `any(reqwest, worker)` for the affected
  API modules. `EndPoint::title()`/`apis()` stay reqwest-only.

Consumers then use the **normal** API:

```rust
let client = google_maps::Client::new(api_key);
let res = client.geocoding().with_address("Ottawa, Canada").execute().await?; // worker::Fetch under the hood
```

A runnable example Worker (geocode, reverse geocode, directions, Places New text search) is in
`examples/worker/`, verified end-to-end under `wrangler dev`.

## Status / verification

- Native default (`reqwest`) build + the full unit-test suite: unchanged, green.
- `wasm32` build with `worker` + the geocoding/directions/places-new-text-search features: green.
- Live `wrangler dev` smoke test: both GET and POST paths round-trip through `worker::Fetch`.

## Questions for you

- Would you accept (a) just the `error.rs` fix, (b) the fix + the `worker` transport, or (c) neither
  (I'll maintain it on the fork)?
- If (b): any preferences on the feature name (`worker`?), and on where the example should live?
- Are you OK taking on the `worker` crate as an optional dependency + a wasm build in CI?

Happy to split this into reviewable PRs whichever way suits you. Thanks again!
