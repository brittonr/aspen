# Aspen auth UCAN adapter

Aspen adopts the sibling `ucan` repository as the source of truth for UCAN
capability-document validation while preserving Aspen's existing token, RPC, CLI,
and federation admission shape.

## Boundary

- `aspen-auth-core` owns Aspen-facing portable types: `Capability`, `Operation`,
  `CapabilityToken`, `Audience`, and `AuthError`.
- `aspen-auth-core` may depend only on sibling `ucan-core`, the `#![no_std]`
  functional core for deterministic UCAN parsing/validation.
- `aspen-auth` owns runtime integration with root `ucan`: capability projection,
  compact-token issuance/verification seams, proof/revocation/replay hooks, and
  shell/readiness adapters.
- Protected `aspen-core --no-default-features` paths must not depend on
  `aspen-auth`, `ucan`, `ucan-core`, or `verified-logic`.

## Current runtime behavior

Aspen still accepts and emits the legacy Aspen `CapabilityToken` wire format.
During runtime verification, `TokenVerifier` validates every presented Aspen
capability by projecting it into a sibling-validated UCAN capability set before
admission proceeds. The existing Aspen authorization rules still decide whether a
verified token authorizes a concrete `Operation`; this preserves current RPC and
CLI behavior while establishing the UCAN validation boundary.

## Resource and ability mapping

The adapter maps Aspen capabilities to UCAN documents using:

- resource: `aspen:<domain>:<scope>`
- ability: `<domain>/<verb>` or `<domain>/*` for Aspen full-scope capabilities

Examples:

| Aspen capability | UCAN resource | UCAN ability |
| --- | --- | --- |
| `Capability::Read { prefix: "tenant-a/" }` | `aspen:kv:tenant-a/` | `kv/read` |
| `Capability::Full { prefix: "tenant-a/" }` | `aspen:kv:tenant-a/` | `kv/*` |
| `Capability::FederationPull { repo_prefix: "forge:org-a/" }` | `aspen:federation:forge:org-a/` | `federation/pull` |
| `Capability::Delegate` | `aspen:auth:` | `auth/delegate` |

The authoritative per-capability table is retained in
`openspec/changes/adopt-sibling-ucan-auth/evidence/i3-aspen-ucan-capability-mapping.md`.

## Dependency policy

`Cargo.toml` pins sibling UCAN by Git revision. Local development against a
checked-out sibling repository is intentionally opt-in through commented
`[patch]` entries in `.cargo/config.toml`; do not enable those patches in
committed configuration.

Nix pins the same source through `ucan-src` and maps Cargo's Git dependency to the
locked flake input during vendoring. If the sibling repository is private, Cargo
and Nix fetches require authorized GitHub SSH access until UCAN is published or
mirrored into an Aspen-owned source cache.

## Migration notes and caveats

- No user-facing token format migration happened in this slice.
- Existing `aspen-token` inspection output and RPC auth-token fields remain
  Aspen `CapabilityToken` based.
- Sibling UCAN compact-token interoperability is not promised until Aspen adds an
  explicit wire-format migration and compatibility rail.
- Aspen-only authorization details remain in the adapter: shell command globs,
  admin implication sets, batch all-item checks, delegation-issuance policy, and
  audit/redaction receipts.
- UCAN validation errors are wrapped as Aspen `AuthError` values so callers do not
  depend on sibling error wording.

## Verification receipts

Focused retained checks for this integration:

```bash
CARGO_TARGET_DIR=target/agent cargo test -p aspen-auth --all-targets
CARGO_TARGET_DIR=target/agent cargo check -p aspen-auth-core --no-default-features
CARGO_TARGET_DIR=target/agent cargo check -p aspen-auth --all-targets
python scripts/check-aspen-core-no-std-boundary.py \
  --manifest-path crates/aspen-core/Cargo.toml \
  --allowlist scripts/aspen-core-no-std-transitives.txt \
  --output /tmp/aspen-core-no-std-current-ucan.txt \
  --diff-output /tmp/aspen-core-no-std-diff-ucan.txt
openspec validate adopt-sibling-ucan-auth --strict
git diff --check
```
