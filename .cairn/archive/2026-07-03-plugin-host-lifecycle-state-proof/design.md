# Design: plugin host lifecycle state proof

## Scope

This change proves plugin lifecycle and hostcall state-machine behavior. It covers manifest install, permission admission, activation/lifecycle callbacks, declared hostcalls, health receipts, upgrade receipts, removal receipts, cleanup, and supply-chain refs.

## Proof checklist

- **Proof claim**: plugin side effects and hostcalls occur only after manifest, permission, lifecycle, resource, supply-chain, and effect-boundary checks pass; failed health, stale manifests, or missing cleanup deny further use or upgrade.
- **Out of scope**: plugin code semantic correctness and ABI implementation proofs beyond inspection/preflight evidence.
- **Trusted assumptions**: Wasm/Steel/external executor preflight receipts are validated by their own gates.
- **Positive evidence**: install→permission→activate→hostcall→health→upgrade/remove traces with matching plugin id, ABI, artifact, policy, resource, and supply-chain refs.
- **Negative evidence**: missing permission, undeclared hostcall, wrong ABI, stale supply-chain ref, failed health, unauthorized namespace, and incomplete cleanup deny before side effects.
- **Canonical refs**: plugin manifest ref, permission receipt ref, lifecycle receipt refs, hostcall receipt refs, health refs, upgrade/removal refs, supply-chain refs, and cleanup refs.
- **Regeneration command**: `cargo test plugin`.

## Functional core

Expose pure lifecycle transition checks over plugin state and candidate receipts. Hostcall execution, file access, and adapter calls remain imperative shells gated by passing pure decisions.

## Non-goals

- No host filesystem access by default.
- No permission inference from manifest presence alone.
