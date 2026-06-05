# CLAUDE.md — google_maps (joeblew999 fork)

Fork of [`leontoeides/google_maps`](https://github.com/leontoeides/google_maps) (v3.9.6) that adds
an **additive Cloudflare Workers (`wasm32`) transport** behind a `worker` feature, plus a small
upstream bug fix. Remotes: `origin` = this fork, `upstream` = leontoeides.

## The one rule that governs every change here

**The native `reqwest` path must stay byte-for-byte equivalent to upstream.** The whole point of
the fork is to be *upstreamable*, so anything Cloudflare-specific is added behind `#[cfg]` gates,
never by changing the default behaviour. After any change, both of these must stay green:

```
mise run test          # native unit tests (256) + wasm worker build gate
mise run test:cf-smoke  # live wrangler dev smoke test (opt-in, needs network)
```

## Architecture of the port

The crate's send path is centralised, which is what made this clean:

- `query_string()` / `query_url()` (the `QueryString`/`QueryUrl` traits) are transport-agnostic — no reqwest.
- Each API's `execute()` is a one-line delegate to `Client::get_request()` / `post_request()`.
- Those central methods are the **only** reqwest callers.

The port:

- **`worker` feature** = `["dep:worker", "dep:http"]`. Also added `dep:http` to the `reqwest`
  feature so both share the same header type (`reqwest::header::HeaderMap` *is* `http::HeaderMap`).
- **`RequestHeaders` trait** now returns `http::HeaderMap` (was `reqwest::header::HeaderMap`),
  gated `any(reqwest, worker)`.
- **New transport files** under `src/client/`, gated `cfg(all(worker, not(reqwest)))`
  (reqwest wins if both on): `worker_transport.rs` (shared helpers), `get_request_worker.rs`,
  `post_request_worker.rs`. No tokio rate-limiter, no `backon` retry.
- **New error variants** `Error::Worker(String)` and `Error::WorkerHttp { status, body }`
  (worker-gated) in `src/error.rs`, plus matching `classify()` arms.
- **Widened cfg gates** from `feature="reqwest"` to `any(reqwest, worker)` for the geocoding,
  directions, and places-new-text-search `execute` / `request_headers` modules and the
  text_search `with_client` / `From` glue. **`EndPoint::title()`/`apis()` stay reqwest-only**
  (they're gated in the trait itself — the worker transport never calls them).
- **Upstream bug fix** in `src/error.rs`: the `HttpWithBody` variant was ungated while its field
  type `HttpErrorStatus` and its `classify()` arm were reqwest-gated, breaking *all*
  `--no-default-features` builds. Added `#[cfg(feature = "reqwest")]` to the variant.

## ConnectRPC layer (`crates/connectrpc`)

A reusable workspace crate, `google-maps-connectrpc`, exposes the worker transport over
**typed Connect RPCs** so any workers-rs project can link it and a generated TS/React client can
call it without drift. Slim `proto/maps/v1/maps.proto` (currently `Geocode` + `TextSearch`) →
`connectrpc-build` (`build.rs`) for Rust, and ready for `buf`/`protoc-gen-es` for TS. `MapsServer`
holds a `google_maps::Client` and impls the generated `MapsService`; mount it with
`Arc::new(MapsServer::new(client)).register(RpcRouter::new())`. Uses `.into_send()` because the
fork's fetch futures are `!Send` and connectrpc 0.4 needs `Send`.

Key structural points:
- The repo is now a **workspace** (`[workspace] default-members = ["."]`) so bare native
  `cargo build`/`test` build ONLY `google_maps` (256 tests, no wasm-only crate); the ConnectRPC
  crate is built explicitly (`-p google-maps-connectrpc --target wasm32`). Examples are `exclude`d
  (standalone). The `[workspace]` block is fork-only — keep it out of the upstream worker-feature PR.
- `protoc` is a mise tool (needed by `connectrpc-build`).
- `connectrpc`/`buffa` versions pinned to match the joeblew999 connectrpc stack
  (`cf-do-locator`, `cf-connectrpc-middleware`) so consumers vendor both cleanly.
- Web/React/Kumo is deferred. The Kumo+React+ConnectRPC reference is
  `cf-connectrpc-middleware/.src/example-multitenant-worker/web-kumo/` (run `kumo ai` before touching
  Kumo). There is NO shared web shell package — the reusable web wheels are the npm packages
  (`@cloudflare/kumo`, `@connectrpc/connect-web`) + codegen; the demo app itself isn't a library.

## Feature sets

- Verified-good wasm set (`WASM_FEATURES` in `mise.toml`):
  `geocoding,directions,places-new-core,places-new-text-search`.
- **Avoid on wasm:** the `places-new` umbrella, `places-new-autocomplete`,
  `places-new-place-details` — they pull `uuid` v4, which hard-errors on `wasm32` without a `js`
  RNG. (If ever needed, the *consumer* crate adds `uuid = { version = "1", features = ["js"] }`.)
- `tokio` appears in the wasm dep tree, but transitively via the `worker` crate (its
  wasm-compatible subset) — unrelated to the tokio-*timer* problem the port avoids.

## Testing — what "test both" actually means

The Cloudflare transport executes in a JS/wasm runtime, so it can't run under native `cargo test`.
Coverage is split:

- **Native (`mise run test:native`):** the 256 unit tests. These cover the *shared* logic — types,
  serde, query-string building — that both transports use identically.
- **Cloudflare build gate (`mise run test:cf`):** compiles the lib + `examples/worker` for wasm32
  with the `worker` feature. Catches any transport/cfg breakage.
- **Cloudflare live smoke (`mise run test:cf-smoke`):** boots `wrangler dev` with a *dummy* key and
  asserts both the GET path (legacy geocoding → `REQUEST_DENIED` in a 200 body) and the POST path
  (Places New → HTTP 400) round-trip through `worker::Fetch`. Needs network egress to googleapis.

## Build / run cheatsheet

`mise tasks` shows only the **user-facing** surface; dev-loop tasks are `hide = true`
(`mise tasks --hidden` shows everything, and hidden tasks are still runnable by name).

Visible:

```
mise run mise:install     # install all tooling + the wasm32 target
mise run gcloud:setup     # ensure auth + CREATE the GCP project if needed + pin project/account
mise run secret:google    # ensure GOOGLE_MAPS_API_KEY in fnox (auto-provision restricted key, else paste)
mise run billing:list     # list GCP billing accounts (+ OPEN status)
mise run billing:use      # flip the project to a billing account (pin + link if OPEN) — turns Maps APIs on
mise run billing:status   # show the project's billing account + enabled state
mise run billing:open     # open the billing console to create/reopen an account
mise run test             # native tests + wasm worker build gate
mise run test:cf-smoke    # live wrangler smoke test (opt-in, needs network)
mise run example:dev      # local wrangler dev for examples/worker (ensures key first)
mise run example:deploy   # deploy the example worker
mise run example:connectrpc:dev    # local wrangler dev for the ConnectRPC example
mise run example:connectrpc:deploy # deploy the ConnectRPC example
```

Hidden (dev internals): `cargo:*`, `upstream:*`, `cf:check`, `example:build`, `test:native`,
`test:cf`. To hide/unhide a task, toggle `hide = true` in `mise.toml`.

## Secrets & the Google API key

`GOOGLE_MAPS_API_KEY` is managed via `fnox` (keychain), per the repo's `fnox.toml` contract.
**Never** hardcode or log it; always pass `-p keychain` to `fnox set`. Shared `CLOUDFLARE_*` creds
come from the same keychain.

**Deterministic Google target.** `mise run gcloud:setup` makes the GCP target reproducible (not
ambient `gcloud config`): it ensures auth, **creates the project if it doesn't exist**, sets its
display name, and pins `GCP_PROJECT_ID` + `GCP_ACCOUNT` into fnox.

Naming derives from the git repo (`<owner>/<repo>`):
- **Project display name** = the repo name, `_`→`-` (e.g. `google-maps`) — "project name matches repo."
- **Project id** = `<owner>-<repo>` with the prohibited word `google` collapsed to `g`
  (e.g. `joeblew999-gmaps`). GCP project ids may NOT contain `google`, and display names may NOT
  contain `_` — both learned the hard way. A pinned `GCP_PROJECT_ID` in fnox overrides the derived
  default.

Optional `GCP_BILLING_ACCOUNT` (in fnox) gets linked — **Maps API *calls* require an OPEN billing
account**, though provisioning/restricting a key does not. The `billing:*` tasks make this easy:
`billing:list` to see accounts, `billing:use` to pin+link one (only links if OPEN — linking a
**closed** account explicitly disables billing and blocks ALL Maps APIs), `billing:status` to check,
`billing:open` to create/reopen in the console. Tunables in `mise.toml` [env]:
`GOOGLE_MAPS_SERVICES` (enabled + restriction targets — must mirror `WASM_FEATURES`),
`GOOGLE_MAPS_KEY_DISPLAY_NAME`.

**Live-verified (2026-06-04):** project `joeblew999-gmaps` (display `google-maps`), restricted key
minted. Places New text search returns **live data** through the worker; geocoding + directions are
correctly key-allowed but blocked by **billing** (all of this account's billing accounts are
closed). The 3-service restriction is confirmed sufficient — every route reached its API past the
key restriction; only billing gates geocoding/directions.

`mise run secret:google` is idempotent: a **no-op if the key already resolves**; otherwise it
auto-provisions a **restricted** key via `gcloud` against the pinned project — enables
`apikeys.googleapis.com` + the Maps services, then reuses-or-creates a key restricted (via
`--api-target`) to exactly `GOOGLE_MAPS_SERVICES`, and stores it in fnox. (`api-keys create`
returns a long-running operation — read `response.name`, never plain `name`.) If gcloud isn't
ready it falls back to a manual paste prompt. `example:dev` depends on it and writes a gitignored
`.dev.vars` from fnox at dev time. GCP config lives in fnox (`GCP_*`); `fnox exec` tolerates these
being unset so they never block dev.

### gcloud via mise — the Python gotcha

`gcloud` is installed by mise via the **asdf** backend (`asdf:mise-plugins/mise-gcloud`; the vfox
backend's installer is broken on macOS). The Cloud SDK needs Python ≥ 3.10 but macOS ships 3.9, so
`mise.toml` declares `python = "3.13"` (only for gcloud's runtime — not project code) and sets
`CLOUDSDK_PYTHON` to the mise python shim. Without that, `mise install` of gcloud fails with
`TypeError: unsupported operand type(s) for |`. On a fresh machine, if `mise install` ever races
python after gcloud, just run it again.

## Conventions

- Tooling is `mise` + `nushell` (OS-neutral; no hardcoded paths). Stack is Rust + nushell — no Python.
- **Do not `git commit`, `git push`, open PRs, or file issues without the user explicitly saying so.**
  The upstream issue announcing Cloudflare support is drafted in `docs/UPSTREAM_ISSUE.md` — file it
  only on request.
