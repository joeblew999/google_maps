<!-- Paste-ready GitHub issue for leontoeides/google_maps. Title = first line; body = the rest. -->

# Roads Speed Limits and the Geolocation API are unreachable from `Client`

Two Google Maps APIs have code in the tree but can't be called on `master` (v3.9.6):

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
