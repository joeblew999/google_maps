# Upstream backlog — leontoeides/google_maps

Things in this fork that should go back to upstream, found while porting to Cloudflare
Workers and building the ConnectRPC + web layer. Each is **additive / independent** — none
changes the default `reqwest` behaviour.

Upstream: https://github.com/leontoeides/google_maps · fork: https://github.com/joeblew999/google_maps

Paste-ready issue drafts live in [`docs/upstream/`](./upstream/) — one file each (title = first line):

| # | Item | Type | Status | Draft |
|---|------|------|--------|-------|
| 1 | Cloudflare Workers (`wasm32`) transport | feature (PR) | **declined** [#44](https://github.com/leontoeides/google_maps/issues/44) (2026-07-12): maintenance burden. He offers to link a separately published worker crate; tracked in joeblew999/google_maps#8 | [`issue-44-cloudflare-workers.md`](./upstream/issue-44-cloudflare-workers.md) |
| A | `--no-default-features` fails to compile (`HttpWithBody` cfg) | **bug** | **fixed in v3.10.0** [#45](https://github.com/leontoeides/google_maps/issues/45) (`2828903`) | [`issue-A-no-default-features.md`](./upstream/issue-A-no-default-features.md) |
| B | `place_photos_uri`/`_image` reject string inputs (`From<Infallible>`) | **bug** | **fixed in v3.10.0** [#46](https://github.com/leontoeides/google_maps/issues/46) (`80b3892`) | [`issue-B-place-photos-infallible.md`](./upstream/issue-B-place-photos-infallible.md) |
| C | Two unreachable endpoints (Roads Speed Limits, Geolocation) | feature gap | **cleaned up in v3.10.0** [#47](https://github.com/leontoeides/google_maps/issues/47): dead `geolocation` module and speed-limit mention removed (`79816af`) | [`issue-C-unreachable-endpoints.md`](./upstream/issue-C-unreachable-endpoints.md) |
| D | `geo` feature doesn't compile with the default Places (New) features (27 errors; also breaks `--all-features`) | **bug** | **filed** [#48](https://github.com/leontoeides/google_maps/issues/48) (2026-10-22) | [`issue-D-geo-feature.md`](./upstream/issue-D-geo-feature.md) |

We merged v3.10.0 into our `main` (joeblew999/google_maps#12) and dropped our own A/B fixes in favour
of upstream's. D isn't in our builds (we don't enable `geo`), so it has no workaround here.

## Verification

**D, verified 2026-10-22 against pristine `upstream/master` (`1c4eed9`, v3.10.0) and v3.9.6** in a
throwaway worktree, rustc 1.99.0: `cargo check --features geo` gives 27 errors in
`src/places_new/types/**/geo.rs` (stale imports: `TypeError`/`GoogleMapsError` not in scope,
private `autocomplete::request::Viewport`, `crate::types::Viewport`, `LocationBias`/`LocationRestriction`).
`--no-default-features --features geo` compiles.

**A–C, verified against pristine `upstream/master` (commit `9ed6a95`, v3.9.6) on 2026-06-06**:

- **A** — `cargo build --no-default-features` fails: `E0425` (cannot find `HttpErrorStatus`) +
  `E0004` (non-exhaustive match on `HttpWithBody`).
- **B** — `client.place_photos_uri("places/X/photos/Y")` fails: `E0277` *"the trait bound
  `GoogleMapsError: From<Infallible>` is not satisfied"*.
- **C1** — `src/roads/mod.rs` says *"(Not yet implemented in this client.)"*; no `Client::speed_limits`.
- **C2** — `src/geolocation/{mod,request,response}.rs` exist, but there is **no `mod geolocation;`
  anywhere** (scanned every `.rs`), so the module is never compiled — no client method, no way to
  reach it, no `geolocation` Cargo feature.

## Monitoring (did he respond?)

`mise run upstream:watch` prints a status table for #44–#48 plus upstream releases and commits. It
**exits non-zero** if anything differs from its per-issue baseline (state + comment count), there's a
new release, or a new commit touches `src/error.rs`, `src/roads` or `src/places_new/types`. Run it
anytime.

Scheduled in CI: **`.github/workflows/upstream-watch.yml`** runs it weekly (Mon 09:00 UTC) and on
demand. A change fails the run, which notifies the repo owner, and the table lands in the job summary.
**It only runs from the default branch (`main`).** Until 2026-10-22 it lived on a non-default branch
and never ran, which is how the #44–#47 replies went unnoticed for 3 months (joeblew999/google_maps#7).
Baselines (v3.10.0, `1c4eed9`, per-issue state/comments) are pinned in the task; bump them after acting
on a change so it goes quiet again.

## This fork's fixes

- Bugs **A** and **B**: fixed upstream in v3.10.0; our patches were dropped when merging it.
- Gaps **C**: upstream removed the dead code rather than implementing it. Speed Limits / Geolocation
  would need real endpoint work here if we ever want them.
- Bug **D**: not patched here; we don't enable `geo`.

## Dependency pins (`main`)

_Last checked: 2026-10-22._ Cross-repo rollout tracker: [cf-connectrpc-middleware#4](https://github.com/joeblew999/cf-connectrpc-middleware/issues/4).

| Dep | We pin | Latest | Source | Notes |
|---|---|---|---|---|
| `connectrpc` | **0.9.1** | 0.9.1 | [anthropics/connect-rust](https://github.com/anthropics/connect-rust) | `crates/connectrpc` forwards a default-on `json` feature; servers with `default-features = false` must enable it or JSON is HTTP 415. |
| `connectrpc-build` | **0.9.0** | 0.9.0 | anthropics/connect-rust | |
| `buffa` | **0.9.2** | 0.9.2 | [anthropics/buffa](https://github.com/anthropics/buffa) | Lockstep with `connectrpc`. |
| `connectrpc-workers` | **0.6.0** | 0.6.0 | [connyay/connectrpc-workers](https://github.com/connyay/connectrpc-workers) | Client worker transport. |
| `worker` | **0.8.7** | 0.8.7 | [cloudflare/workers-rs](https://github.com/cloudflare/workers-rs) | Needs Rust ≥ 1.91 (root crate keeps upstream's 1.76 MSRV otherwise). worker-build 0.8.7 needs wasm-bindgen ≥ 0.2.125 and `strip = "debuginfo"`, not `true`. |

Lockfiles are gitignored, so run `cargo update` in each example after a pull. No crate of ours links against this one, and it doesn't link cf-connectrpc-middleware, so bumps can land any time.
