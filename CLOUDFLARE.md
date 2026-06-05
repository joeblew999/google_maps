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

**REST example:** [`examples/cf-rest-worker/`](examples/cf-rest-worker) — the simple overlay, one HTTP route
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

- Proto: `crates/connectrpc/proto/maps/v1/maps.proto` — `Geocode`, `ReverseGeocode`, `Directions`,
  `TextSearch` (extending it flows to the Rust server, Rust client, and TS client automatically)
- Server runtime: the [`connectrpc`](https://crates.io/crates/connectrpc) crate's `ConnectRpcService`
  — the same approach as `cf-do-locator` and connyay's
  [example-multitenant-worker](https://github.com/connyay/example-multitenant-worker) (the upstream
  reference). connyay's [`connectrpc-workers`](https://github.com/connyay/connectrpc-workers) provides
  the *client* fetch transport for when a Worker needs to *call* a Connect service.
- Mount it: `Arc::new(MapsServer::new(client)).register(RpcRouter::new())`

**Example:** [`examples/cf-connectrpc-worker/`](examples/cf-connectrpc-worker)

```
mise run example:connectrpc:dev      # local wrangler dev
mise run example:connectrpc:deploy   # deploy
mise run test:connectrpc             # smoke the SAME RPC on BOTH native + Cloudflare
```

**Runs natively too.** The crate is feature-gated (`worker` default / `native`) — the *same* proto +
`MapsServer` serves on Cloudflare (`worker::Fetch`) or natively via axum
([`examples/native-connectrpc`](examples/native-connectrpc), using connect-rust's
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

### Why this keeps deployments small (the composition win)

The point of binding over Connect RPC instead of embedding the Maps client: **only the one shared
maps worker carries `google_maps`** — every consumer is tiny. Real release sizes (`mise run
example:connectrpc:sizes`):

| Cloudflare worker (release wasm) | embeds `google_maps`? | size |
|---|---|---|
| `cf-connectrpc-client-worker` — a consumer that **binds over RPC** | **no** | **427 KB** |
| `cf-connectrpc-worker` — the shared maps **server** | yes | 982 KB |
| `cf-rest-worker` — maps embedded directly (REST) | yes | 1.2 MB |
| `native-connectrpc` — native server *binary* | yes | 9.7 MB |

A project worker that just *calls* maps is **~427 KB** — under half the server, with `google_maps`
entirely absent. Scale that across many projects and you save a lot of edge bundle.

### How a dev composes it

- **Rust consumer (Cloudflare or native):** `google-maps-connectrpc` with `features = ["client"]`
  → `MapsServiceClient` + connect-rust's worker/native client transport. **No `google_maps`.**
  Example: [`examples/cf-connectrpc-client-worker`](examples/cf-connectrpc-client-worker).
- **TypeScript consumer (browser or worker):** `@joeblew999/google-maps-connect` →
  `createMapsClient(url)`. Example: [`web/demo`](web/demo) (browser).
- **Compose either way:** point any client at the **shared maps worker** directly, or at a **project
  worker that wraps/extends** it (e.g. adds auth, caches, merges with the project's own RPCs).

## 3. Web — reusable typed client (simple, no Kumo)

[`web/packages/connect`](web/packages/connect) (`@joeblew999/google-maps-connect`) is a
**framework-agnostic** typed Connect client generated from the *same* `maps.proto`.
`createMapsClient(baseUrl)` returns a typed client usable from a browser, Node, or another Worker.
A minimal vanilla-TS demo is in [`web/demo`](web/demo) — no React, no Kumo.

```
mise run web:dev      # vite dev (enter a worker URL in the page)
mise run web:build    # regenerate client + build
mise run web:gen      # regenerate the client from maps.proto
```

**Composition (the point):**
- **worker → worker:** another Rust project (CF or native) calls the maps worker via the generated
  *Rust* Connect client (connect-rust's worker client transport for CF-to-CF, native client otherwise).
- **web → worker:** point `createMapsClient(url)` at the **shared maps worker directly**, or at a
  **project worker that composes** maps into its own API.

**Deferred:** a Kumo + React component layer on top, once the shared Kumo+ConnectRPC setup matures.
The client is intentionally UI-free so any framework reuses it; Kumo is an additive layer, not a
prerequisite.

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
