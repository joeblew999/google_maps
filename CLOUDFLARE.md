# Google Maps on Cloudflare

What this fork runs on **Cloudflare Workers** — the capabilities, how to run them, and what's next.
(For dev internals, gotchas, and the task board, see [CLAUDE.md](CLAUDE.md).)

The whole `google_maps` client runs on `wasm32-unknown-unknown` inside a Worker via an additive
`worker` feature that swaps the native `reqwest` transport for [`worker::Fetch`](https://crates.io/crates/worker).
The native (reqwest) build is unchanged — same `Client` + `.execute()` API on both.

---

## 1. Every Google Maps API, on Cloudflare

All supported APIs run through the Worker transport (3 shared paths: GET, POST, binary):

| Area | APIs |
|------|------|
| Geocoding | forward + reverse |
| Directions | directions |
| Distance Matrix | distance matrix |
| Elevation | positional |
| Time Zone | time zone |
| Roads | snap-to-roads, nearest-roads |
| Address Validation | validate, feedback |
| Places (New) | text search, nearby search, autocomplete, place details, place photos |

> Legacy `places` is intentionally not worker-enabled (it collides with Places-New and Google
> deprecated it). Use Places (New).

**REST example:** [`examples/rest-worker/`](examples/rest-worker) — the simple overlay, one HTTP route
per API. This is the minimal "Maps works on Cloudflare" proof.

```
mise run example:dev      # local wrangler dev
mise run example:deploy   # deploy
mise run test:cf-smoke    # live: hit every API route, assert each round-trips via worker::Fetch
```

## 2. ConnectRPC on Cloudflare (typed RPCs)

A **reusable crate**, [`crates/connectrpc`](crates/connectrpc) (`google-maps-connectrpc`), exposes the
Maps transport as typed Connect RPCs so **any workers-rs project can link it** and a generated
TS/React client calls it without type drift.

- Slim proto: `crates/connectrpc/proto/maps/v1/maps.proto` (currently `Geocode`, `TextSearch`)
- Server runtime: the [`connectrpc`](https://crates.io/crates/connectrpc) crate's `ConnectRpcService`
  — the same approach as `cf-do-locator` and connyay's
  [example-multitenant-worker](https://github.com/connyay/example-multitenant-worker) (the upstream
  reference). connyay's [`connectrpc-workers`](https://github.com/connyay/connectrpc-workers) provides
  the *client* fetch transport for when a Worker needs to *call* a Connect service.
- Mount it: `Arc::new(MapsServer::new(client)).register(RpcRouter::new())`

**Example:** [`examples/connectrpc-worker/`](examples/connectrpc-worker)

```
mise run example:connectrpc:dev      # local wrangler dev
mise run example:connectrpc:deploy   # deploy
mise run test:connectrpc             # smoke the SAME RPC on BOTH native + Cloudflare
```

**Runs natively too.** The crate is feature-gated (`worker` default / `native`) — the *same* proto +
`MapsServer` serves on Cloudflare (`worker::Fetch`) or natively via axum
([`examples/connectrpc-native`](examples/connectrpc-native), using connect-rust's
`into_axum_service()`). One JSON RPC shape, both runtimes.

> **Isomorphic — one contract, everywhere (nice for devs).** A single `maps.proto` is the source of
> truth for *every* side:
> - **Server:** the same `MapsServer` runs native (axum) **and** on Cloudflare (worker) — no rewrite.
> - **Client:** generate from the same proto for the browser (`@connectrpc/connect-web`), native Rust,
>   or another Worker (connect-rust's worker client transport).
> - **Tests:** the identical JSON RPC (`POST /maps.v1.MapsService/Geocode`) hits native *or* edge — so
>   `mise run test:connectrpc` smoke-tests both with one shape.
>
> You write against one typed interface; it works server↔client and native↔edge↔browser, with no drift.

Consumers (e.g. **remy-sport** and other joeblew999 projects) depend on it via Cargo:
```toml
google-maps-connectrpc = { git = "https://github.com/joeblew999/google_maps" }
```

## 3. Cloudflare Kumo + React (soon)

The proto is codegen-ready for a typed TypeScript/React client. The plan is a `web-kumo/` page using
`@cloudflare/kumo` + `@connectrpc/connect-web` against types generated from `maps.proto` — reusing the
npm packages + codegen, no app copying. Reference shape:
`cf-connectrpc-middleware/.src/example-multitenant-worker/web-kumo/`. (Not built yet.)

---

## Getting an API key (turnkey, via gcloud — no Console clicking)

`GOOGLE_MAPS_API_KEY` lives in `fnox` (keychain). The key is **restricted** to only the Maps services
the worker uses.

```
mise run gcloud:setup     # auth + CREATE the GCP project if needed + pin project/account (idempotent)
mise run secret:google    # provision a restricted key via gcloud (or paste one) → stored in fnox
```

**Billing:** Maps API *calls* need an **OPEN** billing account (provisioning a key does not).
```
mise run gcloud:billing:list     # accounts + OPEN status
mise run gcloud:billing:open     # console to create/reopen an account
mise run gcloud:billing:use      # pin + link one (only links if OPEN; a closed account blocks all Maps APIs)
mise run gcloud:billing:status   # current state
```

## Status (2026-06-04)

- Native build + 256 unit tests: green. All wasm builds (lib, ConnectRPC crate, both example workers): green.
- Live smoke: all REST API routes round-trip via `worker::Fetch`; Places text search returned real data.
- **Blocker for live data:** this account's billing accounts are all closed — geocoding/directions
  return "enable billing" until one is reopened (`gcloud:billing:open` → `gcloud:billing:use`). The transport,
  key, and restriction are all proven correct.
