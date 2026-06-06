# Upstream backlog — leontoeides/google_maps

Things in this fork that should go back to upstream, found while porting to Cloudflare
Workers and building the ConnectRPC + web layer. Each is **additive / independent** — none
changes the default `reqwest` behaviour.

Upstream: https://github.com/leontoeides/google_maps · fork: https://github.com/joeblew999/google_maps

| # | Item | Type | Status | Where |
|---|------|------|--------|-------|
| 1 | Cloudflare Workers (`wasm32`) transport | feature (PR) | **filed** [#44](https://github.com/leontoeides/google_maps/issues/44) | `docs/UPSTREAM_ISSUE.md` |
| 2 | `--no-default-features` fails to compile (`HttpWithBody` cfg) | **bug** (PR) | ✅ verified · ready | `src/error.rs` |
| 3 | `place_photos_uri`/`_image` reject string inputs (`From<Infallible>`) | **bug** (PR) | ✅ verified · ready | `src/error.rs` |
| 4 | Roads **Speed Limits** not implemented | feature gap | ✅ verified · ready | `src/roads/` |
| 5 | **Geolocation** module orphaned (never declared in `lib.rs`) | feature gap | ✅ verified · ready | `src/geolocation/` |

Items 2 & 3 are small, standalone bug-fix PRs that stand on their own merit regardless of whether
upstream wants the Workers transport. Items 4 & 5 are "API is incomplete" reports (optionally with
a PR).

> **Verified against pristine `upstream/master` (commit `9ed6a95`, v3.9.6 — identical to our `master`)
> on 2026-06-06** via a throwaway worktree, so every claim below reflects current upstream, not our
> patched fork:
> - **#2** — `cargo build --no-default-features` fails: `E0425` (cannot find `HttpErrorStatus`) +
>   `E0004` (non-exhaustive match on `HttpWithBody`).
> - **#3** — `client.place_photos_uri("places/X/photos/Y")` fails: `E0277` *"the trait bound
>   `GoogleMapsError: From<Infallible>` is not satisfied"*.
> - **#4** — `src/roads/mod.rs` says *"(Not yet implemented in this client.)"*; no `Client::speed_limits`.
> - **#5** — `src/geolocation/{mod,request,response}.rs` exist, but **no `mod geolocation;` anywhere**
>   (scanned every `.rs`), so the module is never compiled — there's no client method *and no way to
>   reach it*, and no `geolocation` Cargo feature.

---

## 1. Cloudflare Workers transport (FILED — #44)

Full draft in [`UPSTREAM_ISSUE.md`](./UPSTREAM_ISSUE.md). An optional `worker` feature, mutually
alternative to `reqwest`, that supplies the central `get_request`/`post_request` via `worker::Fetch`.
Additive, feature-gated, default build unchanged. Includes a runnable example Worker.

## 2. BUG — `--no-default-features` doesn't compile (`HttpWithBody`)

In `src/error.rs` the `HttpWithBody` variant was ungated while its field type `HttpErrorStatus` and
its `classify()` arm were `#[cfg(feature = "reqwest")]` → E0412 + E0004 without `reqwest`. One-line
fix: gate the variant to match. Already described in #44 §1; could land as its own PR.

## 3. BUG — generic `place_photos` bound rejects string inputs

**Repro (upstream, any build):**
```rust
let client = google_maps::Client::new(key);
// Documented usage — does NOT compile:
let photo = client.place_photos_uri("places/XXX/photos/YYY")?.max_width_px(400).execute().await?;
```
**Why:** both `place_photos_uri` and `place_photos_image` are bounded
`where P: TryInto<PhotoRequest>, P::Error: Into<crate::Error>`. For a `From`-based conversion
(`&str`/`String`/`PhotoRequest` → `PhotoRequest`) the associated `TryInto::Error` is
`std::convert::Infallible`, and `crate::Error: From<Infallible>` is **not** implemented — so the bound
is unsatisfiable and the call won't compile. In practice the photo methods are only callable via the
`TryFrom<&Place>` path, not with a photo resource-name string (which is what the API actually returns
in `Place.photos[].name`).

**Fix (this fork, `src/error.rs`):**
```rust
impl std::convert::From<std::convert::Infallible> for Error {
    fn from(value: std::convert::Infallible) -> Self { match value {} }
}
```
Zero-cost (`Infallible` is uninhabited), additive, and makes the documented string-input usage work.

## 4. FEATURE GAP — Roads Speed Limits not implemented

`src/roads/mod.rs` documents Speed Limits but the comment says *"(Not yet implemented in this
client.)"* — there is no `Client::speed_limits(...)`. (Note: Google restricts this endpoint to Asset
Tracking / Premium licenses, so it can't be smoke-tested on a standard key — worth calling out in
any PR.) Report as a tracked gap; optionally implement following the `snap_to_roads` shape.

## 5. FEATURE GAP — Geolocation module is orphaned (never compiled)

`src/geolocation/` defines `Request` (`consider_ip`, cell towers, wifi APs) and `Response`
(`location` + `accuracy`), **but `geolocation` is never declared as a module** — there is no
`mod geolocation;` anywhere in the crate (verified by scanning every `.rs` on `upstream/master`),
no `Client::geolocation()` method, and no `geolocation` Cargo feature. So the files are dead code:
the Geolocation API can't be called *and the module isn't even built*. Report as a gap; the minimal
fix is to declare the module + add a `Client::geolocation()` returning a request whose smallest
useful form is `consider_ip = true`.

---

### How these map to this fork's commits
- Workers transport + bug #2: the bulk of the `feat/cloudflare-workers` branch (see #44 draft).
- Bug #3 (`From<Infallible>`): `0081c18`.
- Gaps #4/#5: observations only — not implemented here (would require real endpoint work in the fork,
  not just the ConnectRPC facade).
