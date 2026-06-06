# CLAUDE.md — google_maps (joeblew999 fork) — DEV notes & task board

Dev-facing notes: architecture, gotchas, conventions, and the TODO/tracking board.
**For *what runs on Cloudflare* and how to operate it, see [CLOUDFLARE.md](CLOUDFLARE.md).**

Fork of [`leontoeides/google_maps`](https://github.com/leontoeides/google_maps) (v3.9.6) adding an
additive Cloudflare Workers (`wasm32`) transport + a reusable ConnectRPC layer. Remotes:
`origin` = this fork, `upstream` = leontoeides. Work lands on branch `feat/cloudflare-workers`.

## The one rule that governs every change

**The native `reqwest` path must stay byte-for-byte equivalent to upstream.** All Cloudflare-specific
code is behind `#[cfg]` gates; never change default behaviour. After any change, keep green:

```
cargo test            # native, 256 unit tests (workspace default-members = ["."], so library only)
mise run test:cf      # wasm build gate: lib + ConnectRPC crate + both example workers
```

## Architecture of the worker port

The send path is centralised — that's what made it clean:
- `query_string()`/`query_url()` traits are transport-agnostic (no reqwest).
- Each API `execute()` delegates to `Client::get_request()`/`post_request()`/`get_binary_request()`.
- Those central methods are the only transport callers.

Port mechanics:
- `worker` feature = `["dep:worker", "dep:http"]`; `reqwest` also gains `dep:http`. Both share
  `http::HeaderMap` (it *is* `reqwest::header::HeaderMap`). `RequestHeaders` returns `http::HeaderMap`.
- Worker transports in `src/client/`, gated `cfg(all(worker, not(reqwest)))` (reqwest wins if both):
  `worker_transport.rs`, `get_request_worker.rs`, `post_request_worker.rs`, `get_binary_request_worker.rs`.
  No tokio rate-limiter / `backon` retry (Workers have no timer).
- `Error::Worker` / `Error::WorkerHttp` variants (worker-gated) + `classify()` arms.
- Across every API, `#[cfg(feature="reqwest")]` on `execute`/`get`/`request_headers` (and the
  Places-New `request_with_client`/`From`/`with_client` glue) widened to `any(reqwest, worker)`.
  **`EndPoint::title()`/`apis()` stay reqwest-only** (gated in the trait; worker transport never calls them).
- Upstream bug fix in `src/error.rs`: gated the `HttpWithBody` variant behind `reqwest` (it was
  ungated while its field type + `classify()` arm were gated → broke all `--no-default-features` builds).

## ConnectRPC layer (`crates/connectrpc`) — and the native+Cloudflare plan

`google-maps-connectrpc` wraps the transport in typed Connect RPCs. `MapsServer` holds a
`google_maps::Client` and impls the generated `MapsService`.

**Server is split a file-per-feature** under `src/server/` (mirroring upstream `src/<feature>/`):
`server/mod.rs` has `MapsServer` + the `MapsService` trait impl, which is a thin dispatch table —
each method delegates to a free fn in a feature module (`geocoding.rs`, `directions.rs`,
`distance_matrix.rs`, `elevation.rs`, `time_zone.rs`, `places.rs`). A Rust trait impl can't span
files, so the dispatch stays central while logic lives per feature. Adding an RPC = new feature module
+ one delegating line in mod.rs. The `exec_await!` macro lives at the bottom of mod.rs (after the `mod`
decls, so it's not textually in their scope) and is pulled into each feature module via `use
super::exec_await;`. **The proto stays a single `maps.proto`** — it's the one GUI-facing contract, and
a protobuf `service` must live in one file anyway.

Server runtime = the `connectrpc`
crate's `ConnectRpcService` (the right one for serving on CF — same as connyay's examples;
`connyay/connectrpc-workers` is the *client* transport, not needed for serving). The `connectrpc`
crate is [`anthropics/connect-rust`](https://github.com/anthropics/connect-rust) — pinned to **0.6.x**
(latest; serves both native via `axum` and CF Workers from one crate). Uses `.into_send()` because
the fork's fetch futures are `!Send` and connectrpc requires `Send`.

**Native + Cloudflare from one proto (DONE).** The crate is feature-gated: `worker` (default,
wasm/`worker::Fetch`) or `native` (`reqwest`). One proto, one `MapsServer`; the only code diff is the
`exec_await!` macro that adds `.into_send()` on the worker path (its fetch futures are `!Send`; native
is already `Send`). `examples/native-connectrpc` serves the SAME service via axum
(`router.into_axum_service()` + `axum::serve`) beside `examples/cf-connectrpc-worker` (CF). `mise run
test:connectrpc` smokes BOTH runtimes with the identical JSON RPC. The middleware crates in
`cf-connectrpc-middleware` (cedar/metrics/rate-limit/tracing) are NOT added yet (too new); we'll bump
that repo to connectrpc 0.6 after all Maps APIs are covered over Connect RPC.

## Workspace & native isolation

Repo is a workspace: `[workspace] members = ["crates/connectrpc"]`, `default-members = ["."]`,
`exclude = ["examples/cf-rest-worker", "examples/cf-connectrpc-worker"]`. So bare `cargo build`/`test`
build only `google_maps` (the wasm-only ConnectRPC crate would fail natively — `worker` is wasm-only).
The `[workspace]` block is fork-only — keep it out of any upstream worker-feature PR.

## Dev gotchas (learned the hard way)

- **uuid on wasm:** `places-new-autocomplete` + `places-new-place-details` pull `uuid` v4 → the
  *consumer* crate must add `uuid = { version = "1", features = ["js"] }` (examples do). `WASM_FEATURES`
  (lib gate) excludes them for this reason; the examples cover them.
- **protoc** is a mise tool — `connectrpc-build` needs it.
- **connectrpc codegen** needs `serde` + `http`/`http-body`/`http-body-util`/`bytes` in the crate.
- **nushell string interpolation:** literal `(...)` inside `$"..."` is parsed as interpolation — use
  brackets or `\(`.
- **gcloud via mise:** use the **asdf** backend (vfox is broken on macOS); Cloud SDK needs Python ≥3.10
  but macOS ships 3.9 → `python = "3.13"` tool + `CLOUDSDK_PYTHON` shim. Re-run `mise install` if it
  races python after gcloud.
- **legacy `places` ⨯ places-new:** both define `Client::text_search`/`nearby_search` — can't coexist.
- **GCP:** project ids can't contain `google`; display names can't contain `_`; `api-keys create`
  returns a long-running op (read `response.name`, not `name`).

## Dev cheatsheet

`mise tasks` shows the user-facing surface; dev-loop tasks are `hide = true` (`mise tasks --hidden`).
- `mise run proto:gen` — regenerate Rust (build.rs) **and** TS (buf) from `maps.proto` in one shot
- `mise run proto:lint` — `buf lint` the proto contract (STANDARD: unique req/resp per RPC)
- `cargo test` — native unit tests (library only)
- `mise run test` — native tests + wasm build gate
- `mise run test:cf` / `test:cf-smoke` / `test:cf-live` — wasm build gate / dummy-key smoke / real-key live
- `mise run test:connectrpc` — token-gate smoke on BOTH runtimes (no-token→unauthenticated, valid→Google); dummy key so it never spends
- `mise run test:connectrpc:deployed` — **billing safety net:** probes the LIVE deployed worker with no-token + bad-token, fails if either is NOT rejected (both are negative → $0 to Google). Also runs automatically at the end of `example:connectrpc:deploy`.
- `cargo:check:wasm`, `cargo:lint`, `cargo:format`, `cargo:machete`, `cargo:pre-commit` (hidden)
- `upstream:fetch` / `upstream:sync` (hidden) — keep master tracking upstream; rebase the feature branch

## Conventions

- Tooling: `mise` + `nushell` (OS-neutral; no hardcoded paths). Stack: Rust + nushell — no Python for code.
- Git: work lands on `feat/cloudflare-workers`; `master` stays = upstream for clean syncing. The user
  has approved ongoing pushes to the feature branch; still ask before pushing to `master` or opening PRs.
- Upstream issue (Cloudflare support) filed: leontoeides/google_maps#44. Draft in `docs/UPSTREAM_ISSUE.md`.

## TODO / tracking

- [x] Worker transport for all APIs + per-API REST example + live smoke
- [x] Turnkey gcloud key provisioning + billing tasks (fnox)
- [x] Reusable `google-maps-connectrpc` crate + Cloudflare example
- [x] **Native + Cloudflare dual-target for the ConnectRPC crate** (feature-gated; `connectrpc-native` axum example; `test:connectrpc` smokes both)
- [x] **Reusable web client (simple, no Kumo)** — `web/packages/connect` (framework-agnostic typed Connect client from maps.proto) + vanilla `web/demo`; mise `web:*`
- [x] **Proto now covers every callable base-crate API (14 RPCs):** Geocode, ReverseGeocode, Directions, Elevation, TimeZone, TextSearch, DistanceMatrix, PlacesAutocomplete, PlacesNearby, **PlaceDetails, PlacePhotos, SnapToRoads, NearestRoads, ValidateAddress**. `Place` carries lat/lng + place_id + phone/website/rating/photo_names; `Route` carries numeric distance_meters/duration_seconds. All 14 verified live in the web GUI. Server split file-per-feature under `server/`. **Not exposed — and not exposable as a facade — because the base crate has no `Client` method for them:** roads `speed_limits` (commented "not yet implemented") and `geolocation` (module exists, no client method). Adding those = real endpoint work in the fork first.
  - Notes: DistanceMatrix/Roads take coordinate `LatLng` lists (Waypoint/path only convert from LatLng), so clients geocode addresses first. autocomplete + place_details pull `uuid` → crate enables uuid's wasm `js` backend only on the `worker` feature. PlacePhotos needs a photo name from a place's `photo_names` (PlaceDetails surfaces them). Upstreamable fix added in `src/error.rs`: `From<Infallible> for Error` (makes the generic `place_photos_uri`/`_image` string-input bound satisfiable).
- [ ] **Kumo + React** layer on top of the web client (later, when the shared Kumo+ConnectRPC setup matures)
- [ ] Publish `@joeblew999/google-maps-connect` to npm (for cross-repo consumers like remy-sport) — currently workspace-only; add a dist build + publish task
- [ ] Bump `cf-connectrpc-middleware` to connectrpc 0.6 (AFTER all Maps APIs covered over Connect RPC)
- [ ] (later) wire `cf-connectrpc-middleware` layers (cedar/tracing/…) once they stabilise
- [x] **Key restrictions synced to all 8 services** — the existing fnox key was updated in place (`api-keys update`, key string + fnox unchanged); `secret:google`'s reuse path now runs `api-keys update …$api_targets` so the restrictions always track `GOOGLE_MAPS_SERVICES` (no re-mint needed when widening).
- [~] Token/quota system (docs/TOKENS.md). DONE: reusable `TokenAuthLayer` (generic → gates BOTH CF worker AND native axum via `MAPS_TOKENS`; `test:connectrpc` proves both no-token→unauthenticated / valid→Google); Rust client sends its token (`MAPS_TOKEN`); `secret:maps-token` issuance; `gcloud:quota:open` for the daily ceiling. REMAINING: CF rate-limit binding + metrics (cf-binding = CF-only; native needs a tower rate-limiter). Middleware portability documented in CLOUDFLARE.md.
- [x] **Billing live + token-gated worker DEPLOYED** — `google-maps-connectrpc-worker-example.gedw99.workers.dev`: no token → `unauthenticated`, valid Bearer → real Google data. `example:connectrpc:deploy` pushes `GOOGLE_MAPS_API_KEY` + `MAPS_TOKENS` secrets from fnox.
- [ ] **Manual:** set per-API daily quota caps in the console (`mise run gcloud:quota:open`) for a hard spend ceiling (can't be done via gcloud — alpha/interactive only). Token gate is the access guard until then.
- Consumers waiting on this: **remy-sport** + other joeblew999 projects.
