# Upstream backlog — leontoeides/google_maps

Things in this fork that should go back to upstream, found while porting to Cloudflare
Workers and building the ConnectRPC + web layer. Each is **additive / independent** — none
changes the default `reqwest` behaviour.

Upstream: https://github.com/leontoeides/google_maps · fork: https://github.com/joeblew999/google_maps

| # | Item | Type | Status |
|---|------|------|--------|
| 1 | Cloudflare Workers (`wasm32`) transport | feature (PR) | **filed** [#44](https://github.com/leontoeides/google_maps/issues/44) — draft in `UPSTREAM_ISSUE.md` |
| A | `--no-default-features` fails to compile (`HttpWithBody` cfg) | **bug** (PR) | ✅ verified · **draft ready** below |
| B | `place_photos_uri`/`_image` reject string inputs (`From<Infallible>`) | **bug** (PR) | ✅ verified · **draft ready** below |
| C | Two unreachable endpoints (Roads Speed Limits, Geolocation) | feature gap | ✅ verified · **draft ready** below |

A & B are small standalone bug-fix PRs that stand on their own merit regardless of whether upstream
wants the Workers transport. C is one consolidated "these endpoints are unreachable" report.

> **Verified against pristine `upstream/master` (commit `9ed6a95`, v3.9.6 — identical to our `master`)
> on 2026-06-06** via a throwaway worktree, so every claim reflects current upstream, not our patched
> fork:
> - **A** — `cargo build --no-default-features` fails: `E0425` (cannot find `HttpErrorStatus`) +
>   `E0004` (non-exhaustive match on `HttpWithBody`).
> - **B** — `client.place_photos_uri("places/X/photos/Y")` fails: `E0277` *"the trait bound
>   `GoogleMapsError: From<Infallible>` is not satisfied"*.
> - **C1** — `src/roads/mod.rs` says *"(Not yet implemented in this client.)"*; no `Client::speed_limits`.
> - **C2** — `src/geolocation/{mod,request,response}.rs` exist, but there is **no `mod geolocation;`
>   anywhere** (scanned every `.rs`), so the module is never compiled — no client method, no way to
>   reach it, no `geolocation` Cargo feature.

This fork's fixes: bug **A** is included in the #44 branch; bug **B** is commit `0081c18`
(`impl From<Infallible> for Error` in `src/error.rs`). The **C** gaps are *not* implemented here
(they need real endpoint work, not just the ConnectRPC facade).

---

# Ready-to-file drafts

Paste-ready. File only on your say-so.

## Issue A (bug PR) — `--no-default-features` doesn't compile

**Title:** `--no-default-features` build fails to compile (`HttpWithBody` / `HttpErrorStatus` cfg mismatch)

**Body:**

On a clean `upstream/master` (v3.9.6), a `--no-default-features` build fails:

```
$ cargo build --no-default-features
error[E0425]: cannot find type `HttpErrorStatus` in this scope
   --> src/error.rs:126:17
error[E0004]: non-exhaustive patterns: `&error::Error::HttpWithBody { .. }` not covered
   --> src/error.rs:205:15
```

**Cause:** in `src/error.rs` the `HttpWithBody` variant is **ungated**, but its field type
`HttpErrorStatus` and the matching `classify()` arm are both `#[cfg(feature = "reqwest")]`. Without
`reqwest` the variant references a type that doesn't exist (E0425) and the `classify()` match becomes
non-exhaustive (E0004).

**Fix:** gate the variant to match its field/arm — one line:

```rust
#[cfg(feature = "reqwest")]   // <-- add
#[error("HTTP error {status}: {body:?}")]
#[diagnostic(code(google_maps::http_with_body))]
HttpWithBody { status: HttpErrorStatus, body: String },
```

Happy to send a PR. (This is the same fix referenced in #44 §1, but it's independent of the Workers
work and can land on its own.)

---

## Issue B (bug PR) — `place_photos_uri` / `_image` can't be called with a string

**Title:** `place_photos_uri` / `place_photos_image` reject `&str`/`String` inputs (missing `From<Infallible> for Error`)

**Body:**

The documented "pass a photo resource name" usage doesn't compile on `upstream/master` (v3.9.6):

```rust
let client = google_maps::Client::new(key);
let _ = client.place_photos_uri("places/XXX/photos/YYY"); // a Place.photos[].name value
```

```
error[E0277]: the trait bound `GoogleMapsError: From<Infallible>` is not satisfied
   --> src/.../uri/request.rs
    = note: required for `Infallible` to implement `Into<GoogleMapsError>`
note: required by a bound in `place_photos_uri`
    |  P::Error: Into<crate::Error>,
```

**Cause:** both `place_photos_uri` and `place_photos_image` are bounded
`where P: TryInto<PhotoRequest>, P::Error: Into<crate::Error>`. For the `From`-based conversions
(`&str` / `String` / `PhotoRequest` → `PhotoRequest`) the `TryInto::Error` is
`std::convert::Infallible`, and `Error: From<Infallible>` isn't implemented — so the bound is
unsatisfiable. In practice these methods are only reachable via `TryFrom<&Place>`, not with the
photo resource-name string the API actually hands you in `Place.photos[].name`.

**Fix:** add a total, zero-cost conversion (`Infallible` is uninhabited):

```rust
impl std::convert::From<std::convert::Infallible> for Error {
    fn from(value: std::convert::Infallible) -> Self { match value {} }
}
```

This makes the string-input call sites work and is purely additive. Happy to send a PR.

---

## Issue C (report / optional PR) — two API endpoints are unreachable

**Title:** Roads Speed Limits and the Geolocation API are unreachable from `Client`

**Body:**

Two Google Maps APIs have code in the tree but can't be called on `upstream/master` (v3.9.6):

**1. Roads — Speed Limits.** `src/roads/mod.rs` documents it but notes *"(Not yet implemented in
this client.)"* — there's no `Client::speed_limits(...)`. (Heads-up: Google restricts this endpoint
to Asset Tracking / Premium licenses, so it can't be smoke-tested on a standard key — worth noting in
any implementation.)

**2. Geolocation.** `src/geolocation/{mod,request,response}.rs` exist and define `Request`
(`consider_ip`, cell towers, wifi access points) and `Response` (`location`, `accuracy`) — but
**`geolocation` is never declared as a module** (no `mod geolocation;` anywhere in the crate), there's
no `Client::geolocation()`, and no `geolocation` Cargo feature. So the module is dead code that isn't
even compiled.

Filing as a tracked gap. Smallest useful fix for Geolocation: declare the module + a
`Client::geolocation()` whose minimal request is `consider_ip = true`. I'm happy to attempt a PR for
Geolocation if you'd take it; Speed Limits is lower-value given the licensing restriction.
