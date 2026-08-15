# retention-bundle-export-profiles

## Summary
Add profile-controlled retention bundle export so operators can distinguish local full-fidelity bundles from public deny-sensitive and diagnostic redacted-review handoffs.

## Motivation
Retention bundles package local deletion evidence for review. Some bundles reference private-secret retention classes, encrypted-ref object kinds, or sensitive records. Operators need a deterministic profile receipt before handing bundles to another person or agent.

## Scope
- Add canonical `retention-candidate-bundle-profile-v1` evidence.
- Add `--profile internal|public|diagnostic` to `molten test retention bundle-export`.
- Keep `internal` as the default full-fidelity local review behavior.
- Make `public` deny when sensitive markers are detected.
- Make `diagnostic` emit redacted review copies plus marker evidence.
- Preserve existing bundle verification for the full-fidelity source bundle.

## Non-Goals
- Granting authority, policy, resource, provenance, transport, execution, source-gate, remote-GC clearance, remote-clearance-import, or deletion trust.
- Replacing retention explain, plan, apply, execute, audit, verification, or destructive admission gates.
- Importing bundle contents into a retention store.
