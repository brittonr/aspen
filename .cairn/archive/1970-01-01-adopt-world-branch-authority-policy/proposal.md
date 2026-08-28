## Why

A world commit can record prior authority observations. Copying that commit cannot copy current authority safely.

Different capabilities need different branch behavior. Some grants can be attenuated. Some are linear. Some are safe only in simulation. Some effects remain blocked until promotion. Others cannot enter a branch.

Basalt must own portable policy meaning. Molten must own current runtime admission, durable transfer state, adapter selection, and effect enforcement.

## What Changes

- Pin a reviewed Basalt cohort that defines world-branch authority policy.
- Add Molten mappings for copyable, attenuated, linear, simulation-only, promotion-gated, replace-before-activation, and non-branchable decisions.
- Build a pure branch-authority derivation plan from supplied capability, policy, scope, and currentness facts.
- Require explicit derivation or transfer evidence. A policy decision does not mint a capability.
- Fence linear transfers through Durable Authority State and prevent simultaneous source and destination activation.
- Replace live adapters with deterministic simulation adapters for simulation-only grants.
- Recheck every required authority fact during branch activation and promotion.
- Keep raw tokens, keys, credentials, secret entropy, and private policy bodies out of commits and receipts.

## Dependencies

- Basalt `add-world-branch-authority-policy`.
- `introduce-world-commit-core` and `add-world-branch-head-protocol`.
- Durable Authority State and UCAN verification.
- Archived `bind-world-promotion-to-effect-release` at Molten revision `0d9dfe4ba9008f3dab1a3c14d5470e8af21a1f4a`.
- Existing Molten capability contexts, runtime authority, effect handles, and simulation adapters.

## Non-Goals

- Treating `capabilities_root` as clonable authority.
- Minting UCANs, moving secrets, discovering keys, or proving global revocation freshness.
- Kernel or hardware capability enforcement.
- Allowing simulation authority to reach live adapters.

## Impact

- **Core**: normalized authority mappings, derivation plans, obligations, activation checks, and diagnostics.
- **Shell**: current authority observation, UCAN derivation, durable linear transfer, simulation adapter binding, promotion reservation admission, and activation rechecks.
- **Schemas**: branch-authority plans, observations, transition receipts, and safe diagnostics.
- **Testing**: each allowed mode plus negative widening, double activation, stale policy, missing derivation, simulation escape, and secret-disclosure cases.
