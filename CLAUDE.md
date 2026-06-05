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
`google_maps::Client` and impls the generated `MapsService`. Server runtime = the `connectrpc`
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
- [~] Expand the proto over Connect RPC (flows to Rust + TS clients via `proto:gen`): **done** — Geocode, ReverseGeocode, Directions, Elevation, TimeZone, TextSearch; **remaining** — distance matrix, roads (snap/nearest), address validation, places-new (nearby / autocomplete / place details / photos)
- [ ] **Kumo + React** layer on top of the web client (later, when the shared Kumo+ConnectRPC setup matures)
- [ ] Publish `@joeblew999/google-maps-connect` to npm (for cross-repo consumers like remy-sport) — currently workspace-only; add a dist build + publish task
- [ ] Bump `cf-connectrpc-middleware` to connectrpc 0.6 (AFTER all Maps APIs covered over Connect RPC)
- [ ] (later) wire `cf-connectrpc-middleware` layers (cedar/tracing/…) once they stabilise
- [ ] Re-mint (or update restrictions on) the EXISTING fnox key — it was minted with only 3 services; GOOGLE_MAPS_SERVICES now lists 8, but secret:google reuses the existing key by name (does not update its api-targets). Delete+re-mint or add a restrictions-update step.
- [~] Token/quota system (docs/TOKENS.md). DONE: reusable `TokenAuthLayer` (generic → gates BOTH CF worker AND native axum via `MAPS_TOKENS`; `test:connectrpc` proves both no-token→unauthenticated / valid→Google); Rust client sends its token (`MAPS_TOKEN`); `secret:maps-token` issuance; `gcloud:quota:open` for the daily ceiling. REMAINING: CF rate-limit binding + metrics (cf-binding = CF-only; native needs a tower rate-limiter). Middleware portability documented in CLOUDFLARE.md.
- [ ] Reopen a billing account to unblock live data for all APIs
- Consumers waiting on this: **remy-sport** + other joeblew999 projects.
