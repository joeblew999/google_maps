<!-- Paste-ready GitHub issue for leontoeides/google_maps. Title = first line; body = the rest. -->

# `--no-default-features` build fails to compile (`HttpWithBody` / `HttpErrorStatus` cfg mismatch)

On a clean `master` (v3.9.6), a `--no-default-features` build fails:

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

Happy to send a PR. (Same fix referenced in #44 §1, but it's independent of the Workers work and can
land on its own.)
