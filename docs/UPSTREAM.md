# Upstream backlog — leontoeides/google_maps

Things in this fork that should go back to upstream, found while porting to Cloudflare
Workers and building the ConnectRPC + web layer. Each is **additive / independent** — none
changes the default `reqwest` behaviour.

Upstream: https://github.com/leontoeides/google_maps · fork: https://github.com/joeblew999/google_maps

Paste-ready issue drafts live in [`docs/upstream/`](./upstream/) — one file each (title = first line):

| # | Item | Type | Status | Draft |
|---|------|------|--------|-------|
| 1 | Cloudflare Workers (`wasm32`) transport | feature (PR) | **filed** [#44](https://github.com/leontoeides/google_maps/issues/44) | [`issue-44-cloudflare-workers.md`](./upstream/issue-44-cloudflare-workers.md) |
| A | `--no-default-features` fails to compile (`HttpWithBody` cfg) | **bug** (PR) | **filed** [#45](https://github.com/leontoeides/google_maps/issues/45) | [`issue-A-no-default-features.md`](./upstream/issue-A-no-default-features.md) |
| B | `place_photos_uri`/`_image` reject string inputs (`From<Infallible>`) | **bug** (PR) | **filed** [#46](https://github.com/leontoeides/google_maps/issues/46) | [`issue-B-place-photos-infallible.md`](./upstream/issue-B-place-photos-infallible.md) |
| C | Two unreachable endpoints (Roads Speed Limits, Geolocation) | feature gap | **filed** [#47](https://github.com/leontoeides/google_maps/issues/47) | [`issue-C-unreachable-endpoints.md`](./upstream/issue-C-unreachable-endpoints.md) |

A & B are small standalone bug-fix PRs that stand on their own merit regardless of whether upstream
wants the Workers transport. C is one consolidated "these endpoints are unreachable" report.

## Verification

**Verified against pristine `upstream/master` (commit `9ed6a95`, v3.9.6 — identical to our `master`)
on 2026-06-06** via a throwaway worktree, so every claim reflects current upstream, not our patched
fork:

- **A** — `cargo build --no-default-features` fails: `E0425` (cannot find `HttpErrorStatus`) +
  `E0004` (non-exhaustive match on `HttpWithBody`).
- **B** — `client.place_photos_uri("places/X/photos/Y")` fails: `E0277` *"the trait bound
  `GoogleMapsError: From<Infallible>` is not satisfied"*.
- **C1** — `src/roads/mod.rs` says *"(Not yet implemented in this client.)"*; no `Client::speed_limits`.
- **C2** — `src/geolocation/{mod,request,response}.rs` exist, but there is **no `mod geolocation;`
  anywhere** (scanned every `.rs`), so the module is never compiled — no client method, no way to
  reach it, no `geolocation` Cargo feature.

## Monitoring (did he respond?)

`mise run upstream:watch` — prints a status table for #44–#47 + upstream release/commits, and **exits
non-zero** if anything changed (a comment, a close, a new release, or a commit touching
`error.rs`/`roads`/`geolocation`). Run it anytime.

Scheduled in CI: **`.github/workflows/upstream-watch.yml`** runs it weekly (Mon 09:00 UTC) + on demand
("Run workflow"). A change flips the task to a failed run → GitHub notifies the repo owner; the status
table is written to the run's job summary. Baselines (`v3.9.6`, commit `9ed6a95`, all issues open &
uncommented) are pinned in the task — bump them after acting on a change so it goes quiet again.

## This fork's fixes

- Bug **A**: included in the `feat/cloudflare-workers` branch (see the #44 draft).
- Bug **B**: commit `0081c18` — `impl From<Infallible> for Error` in `src/error.rs`.
- Gaps **C**: *not* implemented here (they need real endpoint work, not just the ConnectRPC facade).
