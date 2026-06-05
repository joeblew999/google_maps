# Protecting the shared maps worker (tokens, rate limits, quota)

The shared maps Connect worker holds **one restricted Google key**. Many projects + the web demo
call it, so we must stop anyone burning that key's spend. This is the recommended design — chosen to
fit the project goals (one shared service, many consumers, edge-native) and the existing tech base
(connect-rust on CF Workers + the `cf-connectrpc-middleware` tower layers).

## Defense in depth — 4 layers, cheapest-first

1. **GCP per-key daily quota (the hard ceiling).** On the *restricted* Google key in the Cloud
   Console, set **per-API daily request limits**. This caps spend *no matter what* reaches Google —
   independent of the worker. Free, and the safest backstop. Do this first.

2. **Authn — a Bearer token per consumer.** The maps worker requires
   `Authorization: Bearer <token>` and rejects unknown tokens. One token per consuming project
   (e.g. `MAPS_TOKEN_REMYSPORT`), kept in `fnox`/CF secrets. This gates **who** can call.
   - **worker → worker:** the calling worker holds its token server-side (a secret) and sends it via
     a connect-rust client interceptor — safe.
   - **browser → worker:** browsers can't hold a secret. So a browser **never** calls the shared
     worker directly with a token. It calls a **project worker** that holds the token and proxies
     (the composition model), or a dedicated **public demo tier** (see below).

3. **Rate limit — Cloudflare Rate Limiting binding, keyed by token.** Caps per-consumer
   requests/min = caps Google cost and absorbs abuse at the edge. This is exactly what
   [`connectrpc-cf-rate-limit`](../../cf-connectrpc-middleware) is — a `tower::Layer`, no extra infra.

4. **Metrics — `connectrpc-cf-metrics` (Analytics Engine).** Per-token / per-RPC counters so we can
   *see* usage, attribute cost, and spot abuse. Also a `tower::Layer`.

(Later) **Authz — `connectrpc-cedar`.** Policy on *what* each token may do: which RPCs, and caps on
expensive calls (Places field masks, large Distance Matrix). Fine-grained cost control when needed.

## Why this design

- Every layer is a connect-rust `tower::Layer` — they **compose** with the existing `MapsServer`
  with no architectural change. This is precisely what `cf-connectrpc-middleware` was built for.
- Edge-native bindings (Rate Limiting, Analytics Engine) — **no DB, no Durable Object** for the basic
  tier. Cost protection lives where the request already is.
- It layers cleanly on what's already done: the Google key is **already API-restricted**; tokens add
  *who*, rate-limit adds *how much*, GCP quota adds the *hard ceiling*, cedar adds *what* (later).

## The browser / public-demo tier

The public web demo can't carry a token. Two safe options:
- **Proxy (preferred):** the demo calls a small project worker that injects the token + strict rate
  limit, then forwards to the shared worker.
- **Public tier:** a separate route/worker with a very low Cloudflare rate limit + origin check, using
  the restricted key whose **GCP daily quota** is the real cap. Acceptable because layer 1 bounds it.

## Phasing (the middleware crates are still new)

- **Phase 0 — interim, no new deps (ships today):** a tiny hand-rolled token-check `tower::Layer`
  (compare the Bearer against a CF secret set) + call the **CF Rate Limiting binding** directly +
  set **GCP daily quotas**. ~50 lines, zero dependency on the new middleware.
- **Phase 1:** swap the hand-rolled bits for `connectrpc-cf-rate-limit` + `connectrpc-cf-metrics`
  once that middleware stabilises (then bump `cf-connectrpc-middleware` to connectrpc 0.6 — already
  on the roadmap).
- **Phase 2 (if needed):** hard monthly caps via per-token counters in KV/DO; `connectrpc-cedar` for
  per-RPC policy.

## Token issuance / rotation

Tokens are the cross-repo secret contract (see fnox conventions): one keychain item per consumer
(`MAPS_TOKEN_<CONSUMER>`); the maps worker validates against its allowed set (a CF secret or KV list).
Rotating = update the consumer's secret + the worker's allowed set. Never put a token in the browser.

## TL;DR recommendation

Start with **GCP daily quota + a Bearer-token layer + CF Rate Limiting** (Phase 0) — it's small,
edge-native, and bounds spend immediately. Graduate to the `cf-connectrpc-middleware` layers
(rate-limit/metrics/cedar) when they're ready. The browser demo goes through a proxying project
worker, never holds a token.
