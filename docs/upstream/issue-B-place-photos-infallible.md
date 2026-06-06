<!-- Paste-ready GitHub issue for leontoeides/google_maps. Title = first line; body = the rest. -->

# `place_photos_uri` / `place_photos_image` reject `&str`/`String` inputs (missing `From<Infallible> for Error`)

The documented "pass a photo resource name" usage doesn't compile on `master` (v3.9.6):

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
