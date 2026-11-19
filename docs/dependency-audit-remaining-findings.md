# Remaining dependency audit findings

The 2026-11-19 remediation refreshed every registry crate whose advisory had a
semver-compatible fix and took the default closure's reachable vulnerabilities
from 38 to 4
([record](../.cairn/archive/2026-11-19-remediate-default-closure-advisories/evidence/advisory-remediation-2026-11-19.json)).
The audit still denies: 158 findings remain. This page records each remaining
class, who owns it, and what would reopen it. It grants no exception.

## Advisories that need an upstream move

| Finding | Owner | Why it cannot move here |
| --- | --- | --- |
| `rsa@0.9.10` (`>=0.10.0`) | basalt | basalt pins `rsa = "=0.9.10"` and `jsonwebtoken = "=11.0.0"`; jsonwebtoken 11 requires rsa 0.9, so rsa 0.10 needs a jsonwebtoken release that accepts it |
| `crypto-bigint@0.5.5` (`>=0.7.6`) | basalt | reaches the closure through `jsonwebtoken -> p256 -> elliptic-curve`; same pin |
| `imbl-sized-chunks@0.1.3` (`>=0.2.0`) | Nickel pin | `nickel-lang-vector` selects 0.1; 0.2 is semver-breaking and lives in the pinned Nickel source |
| `tracing-subscriber@0.2.25` (`>=0.3.20`) | syndicate | `syndicate 0.44.0-rc.3` selects 0.2; 0.3 is semver-breaking and needs an upstream release |

The `rsa` advisory describes a timing side channel on RSA decryption with a
private key. Basalt's only `rsa` runtime path is `jsonwebtoken` RS256
verification against a public JWKS (`basalt/src/oidc_exchange/core.rs`), so the
advisory's attack path is not exercised. That scoping does not fix the pinned
version and is not a claim that RSA decryption would be safe here.

No `[[advisory-exception]]` is added. An exception requires owner, review and a
90-day expiry; these four are blocked upstream work, and an exception would
convert a known mismatch into silent acceptance. A vendored `[patch]` inside
this workspace is not a substitute either: it would diverge from the pinned
identities the release evidence binds.

## Package-evidence findings

| Finding | Count | Meaning | Closure |
| --- | ---: | --- | --- |
| `unavailable-notices` | 85 | the crate source ships no file starting `LICENSE`, `NOTICE`, `COPYING`, `COPYRIGHT` or `AUTHORS` and declares no `license-file`; the audit retains only local bytes | an upstream release that ships the text |
| `unavailable-license` | 39 | the package declares no valid SPDX expression; a `license-file` alone is not an expression | an upstream SPDX expression |
| `license-policy` | 5 | the declared SPDX expression is outside `allowed_licenses` in the audit policy | a reviewed license choice or an upstream relicense |

The audit tool has no exception path for notice or SPDX findings, by design:
accepting a package without its license text would make the retained notice
bytes a false claim. Closing these classes needs either upstream evidence or a
new reviewed acceptance rule with its own requirement, owner and expiry.

## The pilot's Wasmtime cohort

The archived component-RPC pilot pins `wasmtime`/`wasmtime-wasi` 33.0.0 with wRPC
`aa01409e` and `wrpc-runtime-wasmtime` 0.29.0. Advisories against those versions
are patched only in later cohorts (36+, 45+). The cohort is reachable solely
through the non-default `component-rpc` feature, so the default release closure
this audit scans does not include it.

Moving the cohort is a new qualification, not a version bump: the pilot's WIT
world, transport framing, host callbacks and archived evidence all bind 33.0.0.
The cohort therefore keeps its advisories recorded rather than excepted, and the
pilot stays experimental until it is retired or re-qualified by its own change.

## Upstream ownership

The four advisories were checked against whether any stack repository owns the
fixing crate. None of the four is owned here: `rsa` arrives through
`secretspec 0.17.0`, `crypto-bigint` through `k256 0.13.4`, `imbl-sized-chunks`
through the pinned Nickel source, and `tracing-subscriber` through
`syndicate 0.44.0-rc.3`. The one owned move — Basalt replacing `jsonwebtoken`
with a `ring` RS256 verifier — is recorded in
[upstream ownership](dependency-audit-upstream-ownership.md), together with why
adopting that revision does not change this audit yet.

## Reopening

Reopen the advisory rows when basalt moves jsonwebtoken/rsa, when the pinned
Nickel source moves `nickel-lang-vector` to imbl 0.2, or when syndicate ships a
`tracing-subscriber` 0.3 release. Reopen the evidence rows when an upstream
publishes the missing license text, or when a reviewed SPDX-only acceptance rule
is proposed with its own requirement and expiry. Reopen the cohort row only
through a pilot retirement or re-qualification change.
