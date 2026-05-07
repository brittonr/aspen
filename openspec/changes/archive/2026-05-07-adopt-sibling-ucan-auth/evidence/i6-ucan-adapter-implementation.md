# I6 UCAN adapter implementation

- Change: `adopt-sibling-ucan-auth`
- Task: Implement the UCAN-backed adapter that preserves Aspen-facing `Capability`, `Operation`, token CLI/RPC, and redacted receipt behavior.
- Started: 2026-05-06T23:42:08Z
- Completed: 2026-05-06T23:48:10Z
- Status: PASS

## Implementation

Added `crates/aspen-auth/src/ucan_adapter.rs` and exported it from `aspen-auth`.

The adapter is intentionally narrow:

- It preserves the existing Aspen-facing `Capability`, legacy `CapabilityToken`, builder/verifier, CLI, and RPC behavior for this slice.
- It converts Aspen `Capability` variants into sibling `ucan::shell::CapabilityDocument` values using the mapping recorded in `evidence/i3-aspen-ucan-capability-mapping.md`.
- It builds sibling `ucan::token::CapabilitySet` values for future issuance/verification wiring.
- It lets the sibling `ucan` crate validate resource/ability syntax rather than reimplementing UCAN document validation locally.
- It keeps the actual runtime verifier switch gated behind the remaining compatibility-fixture and negative-evidence tasks.

## Verification

- `nix run .#rustfmt` → PASS
- `CARGO_TARGET_DIR=target/agent cargo test -p aspen-auth ucan_adapter --all-targets` → PASS
  - `maps_kv_full_to_ucan_wildcard_ability` passed.
  - `maps_delegate_as_auth_boundary_marker` passed.
  - `builds_sibling_ucan_capability_set` passed.

## Boundary notes

`Delegate` is represented as `resource=aspen:auth:` / `ability=auth/delegate` only as an Aspen boundary marker. Actual UCAN delegation remains proof-chain based and is not reimplemented in Aspen.

The adapter does not yet switch RPC admission or token CLI parsing to the sibling UCAN verifier. That is intentional: the OpenSpec task sequence requires compatibility fixtures and negative evidence before the verifier switch.
