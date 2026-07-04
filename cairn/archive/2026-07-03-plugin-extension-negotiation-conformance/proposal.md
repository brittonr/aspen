## Why

Once extension contracts are first-class artifacts, Molten needs a fail-closed way to decide which extensions are active for a plugin, whether a host supports the required versions, whether an upgrade preserves contract compatibility, and whether an extension implementation has passed the conformance evidence expected by the contract.

Without explicit negotiation and compatibility receipts, extension adoption risks becoming implicit feature detection or best-effort fallback, both of which would weaken Molten's receipt-backed authority model.

## What Changes

- Add explicit host/plugin extension negotiation with required and optional extension contract refs, supported host feature refs, selected feature refs, and denial diagnostics.
- Add plugin extension compatibility receipts for upgrades that compare old/new contract refs, schema compatibility, retained hostcalls, migration refs, rollback refs, cleanup refs, and conformance evidence.
- Require extension contracts to bind positive, negative, and property/conformance suite refs for production admission.
- Add tests for missing required extension denial, unsafe downgrade denial, compatible upgrade pass, hostcall removal denial, and missing conformance evidence denial.

## Impact

- **Specs**: `plugin-host` gains negotiation, compatibility, and conformance requirements for extension contracts.
- **Core**: activation and upgrade gates become extension-aware pure validation surfaces.
- **Testing**: conformance evidence becomes part of plugin extension admission rather than a separate convention.
