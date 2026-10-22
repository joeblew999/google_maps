`geo` feature doesn't compile with the default Places (New) features (27 errors)

<!-- Filed as leontoeides/google_maps#48 on 2026-10-22. Title = first line. -->

Hi, thanks again for turning #44–#47 around so quickly. One more small one, found while checking v3.10.0.

**`cargo check --features geo` (default features + `geo`) fails with 27 errors.** It's the same on v3.10.0 (`1c4eed9`) and v3.9.6, with rustc 1.99.0. `--no-default-features --features geo` compiles, so it's the `geo` impls under `places_new` that break once the default Places (New) modules are on. That also means `--all-features` doesn't build.

```
cargo check --features geo
```

All errors are in the `geo.rs` files under `src/places_new/types/`:

| file | error |
|---|---|
| `lat_lng/geo.rs` | `TypeError` / `GoogleMapsError` not in scope (16) |
| `lat_lng/geo.rs`, `viewport/geo.rs`, `request/location_restriction/geo.rs` | `use crate::places_new::autocomplete::request::{…Viewport}`: `request` is private and has no `Viewport` |
| `request/location_bias/geo.rs` | `use crate::types::Viewport`: no `Viewport` in `types`; `LocationBias` not in scope |
| `request/location_restriction/geo.rs` | `LocationRestriction` not in scope |

These look like stale import paths from when Places (New) landed. Most should point at `crate::places_new::Viewport` / `LatLng` and the module's own error types. Happy to send a small PR if that helps.
