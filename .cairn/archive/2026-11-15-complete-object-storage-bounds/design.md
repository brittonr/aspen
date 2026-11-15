# Design

## Existing ownership
Reuse Aspen's pure content adapter transition core, SigV4 client and capability-local manifest identity. Object keys remain locators, never canonical chunk identity. The local tier stores raw-byte BLAKE3 entries, while exposure additionally checks the domain-separated chunk ref and manifest length. Internal molten crate/schema names remain unchanged.

## Bounds
A single admitted client permits one outstanding remote request, fails excess concurrency before dispatch, disables proxies and redirects and admits only numeric endpoints. DNS and credential refresh are absent from the request path: endpoints and credentials are fixed for the lifetime of the client; expiration requires separately admitted replacement. No IdP call can hang a request. Request and connect deadlines remain finite. The tier has separate byte and entry caps, idempotent insert accounting, bounded reads and reopen rejection above capacity. At capacity cold reads remain verified remote-only; no automatic eviction.

## Recovery
Rebuild means re-fetch and re-verify hot entries, never rebuild all remote inventory. Recovery capacity is independent of per-operation bytes. At 1 GiB/s sustained verified input, a 64 GiB cap takes 64 seconds of transfer; this is a conditional transfer bound, not an MTTR guarantee (availability, verification throughput and stalls must be bounded independently). Non-rotational is not proof of NVMe. Hardware qualification must identify the actual backing NVMe device and measured rates.

## Failures and authority
Network status and request failures map to existing typed terminals, including throttle retryable and timeout uncertain. PUT errors after dispatch must remain uncertain where remote mutation cannot be excluded; no implicit retries. Deletion requires the existing canonical retention authority decision as well as a local protection guard. A remote lifecycle rule is not controlled by this client: a protected-object remote-presence audit fails closed on missing state.

## Evidence
Executable tests and pinned fixture inputs are source, not executed receipts. Retained prior prose claims are not reissued as current evidence. Issue #13 still requires an executed canonical receipt with fixture command digests; issue #14 source fixtures already exist. No speedup claim.

Parent integration exact-pins the existing optional reqwest 0.13.4, sha2 0.10.9, hmac 0.12.1 and time 0.3.47 cohort with observed existing Cargo.lock checksums in its source policy. This adds no client package or version update. Root locks/build plans and their executed evidence remain parent-owned. The direct-dependency guard and baseline/current lock or inverse-root evidence must be retained after regeneration.

## Lifecycle owner

The delta modifies the existing `content-addressed-chunk-store` capability, which already owns all three selected `molten.content_store_adapter` requirement IDs. Keep the existing verification, resource and retention clauses and scenarios while adding the object-shell refinements. Do not create a parallel `content-store-adapter` capability or relabel these existing requirements as `ADDED` to bypass native merge admission.

