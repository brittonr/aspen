## Why

Many Molten CLI workflows require the same clusters of policy, capability, authority, resource, retention, redaction, and supporting evidence refs. Operators currently repeat those refs across commands, which makes scripts noisy and increases the chance that one command accidentally omits or swaps evidence.

A reviewed operator context profile can make common command contexts reusable while preserving the existing rule that every downstream gate still validates explicit refs for the exact operation.

## What Changes

- Define an operator context profile artifact for reusable policy/resource/capability/authority/evidence/ref groups.
- Allow selected CLI commands to accept a context profile and expand it into normal command inputs before calling pure command cores.
- Bind expanded context refs and profile refs into receipts so reviewers can see both the profile and the actual refs used.
- Deny stale, malformed, over-broad, or operation-incompatible profiles before mutation.
- Add positive and negative tests for context expansion, profile/ref mismatch, missing required context, and attempts to treat profile presence as authority.

## Impact

- **Files**: operator workflow specs, context profile contracts, CLI argument plumbing, command-core input expansion, receipts, and tests.
- **Testing**: pure context expansion tests plus CLI tests on representative node, retention, catalog, and live-send commands.
- **Safety**: operator context profiles are convenience and review evidence only. They do not grant authority, policy admission, resource rights, provenance, source-gate trust, retention clearance, live transport trust, or mutation permission without the expanded refs passing subsystem gates.
