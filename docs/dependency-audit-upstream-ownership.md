# Upstream advisory moves: ownership and outcome (2026-11-19)

Issue #28's remaining advisories were triaged for whether this stack owns the
fixing crate. This page records who owns each one and what running the owned fix
actually bought.

## Ownership

| Advisory | Crate owned here? | Owner of the fix |
| --- | --- | --- |
| `rsa@0.9.10` | no | registry crate behind `secretspec 0.17.0`; needs a secretspec release that accepts rsa 0.10 |
| `crypto-bigint@0.5.5` | no | registry crate behind `k256 0.13.4` (Basalt's `ucan-nostr` uses secp256k1 ECDSA); needs k256 on elliptic-curve 0.14+ |
| `imbl-sized-chunks@0.1.3` | no | nickel-lang-vector in the pinned Nickel source; imbl 0.2 support is upstream Nickel work |
| `tracing-subscriber@0.2.25` | no | `syndicate 0.44.0-rc.3`; needs a syndicate release on tracing-subscriber 0.3 |

The one crate this stack owns in that list was Basalt's `jsonwebtoken`
dependency: `jsonwebtoken 11.0.0` pins `rsa 0.9` and pulls p256/elliptic-curve,
and Basalt used it for exactly one primitive.

## What the owned move changed

Basalt now verifies RS256 with `ring` and keeps compact-JWS splitting, JWK
decoding, header admission, claim decoding and policy in its own core
(`basalt` commit `4b09316`, merged as `264d248` on `main`). 276 tests pass with
all features and clippy is clean. `jsonwebtoken` stays only as a Basalt
dev-dependency for fixture tokens.

Adopting that revision in Molten does **not** reduce the audit:

- `crypto-bigint` is still reachable through `k256` in Basalt's `ucan-nostr`;
- `rsa` is still reachable through `secretspec`, which Basalt depends on for
  issuer-key resolution;
- the audit moved from 158 to 164 findings after the repin because the new
  Basalt revision is not yet bound by the reviewed release-dependency profile
  (`source-policy` 5 -> 10, plus one `prerequisite-unavailable`).

Because the swap bought no advisory reduction and made the source-policy
bindings worse, Molten keeps its current Basalt pin (`89675cd4`). The Basalt
change stands on its own as a narrower crypto surface; adopting it needs a
release-dependency profile revision that binds `264d248` and a regenerated
release-evidence source, which is its own change.

## What would actually close the four advisories

Move `secretspec` to an rsa 0.10-compatible release, move `k256` to a release on
elliptic-curve 0.14+, move `nickel-lang-vector` to imbl 0.2, and move `syndicate`
to tracing-subscriber 0.3. Three are registry crates that do not belong to this
stack; the Nickel one is the pinned upstream source. None of them can be forced
from this workspace without a `[patch]` that would diverge from the pinned
identities the release evidence binds.
