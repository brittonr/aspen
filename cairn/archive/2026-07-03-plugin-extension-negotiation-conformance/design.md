# Design: plugin extension negotiation and conformance

## Scope

This change builds on canonical extension contract refs. It defines how a host and plugin select an admitted extension surface, how upgrades prove extension compatibility, and how conformance suites become evidence for production admission.

## Negotiation model

Activation should use an explicit `plugin-extension-negotiation-receipt-v1` rather than implicit host feature probing.

Inputs:

- active plugin manifest ref;
- required extension contract refs and version/range constraints;
- optional extension contract refs and version/range constraints;
- host-supported extension contract refs or feature refs;
- policy/resource/provenance constraints for selected features;
- denial policy for downgrades or missing required contracts.

Outputs:

- decision;
- manifest ref;
- selected required and optional extension refs;
- denied/missing extension diagnostics;
- host feature snapshot ref;
- checks for fail-closed negotiation and no implicit fallback.

Required extensions must be present and compatible. Optional extensions may be omitted only when policy allows omission and the plugin has an explicit fallback contract that does not broaden authority.

## Compatibility model

Upgrade validation should emit `plugin-extension-compatibility-receipt-v1` before a replacement manifest can become active. The receipt compares old and new extension contract sets.

Compatibility checks include:

- same plugin id and active manifest lineage;
- compatible host ABI;
- retained required extension ids;
- compatible extension versions;
- retained or explicitly migrated hostcall descriptors;
- input schema compatibility;
- output schema compatibility;
- authority/resource/effect requirement compatibility;
- state migration refs when required;
- rollback and cleanup refs;
- passing conformance evidence for new or changed contracts.

## Conformance model

Each production-admitted extension contract should bind conformance suite refs:

- positive suite: valid descriptor use passes;
- negative suite: undeclared, unauthorized, malformed, stale, or downgraded requests deny;
- property suite: bounded determinism, ref stability, and no-ambient-authority invariants.

Production admission may deny when conformance refs are absent, stale, or fail. Development profiles may permit diagnostic-only contracts only when receipts clearly mark them as non-production and non-authority.

## Functional core

Negotiation and compatibility are pure decisions over loaded manifests, extension contracts, host feature snapshots, compatibility inputs, and conformance refs. CLI or runtime shells load values, write receipts, and perform any mutation only after pure pass decisions.

## Non-goals

- No network extension discovery protocol.
- No automatic semantic-version solver beyond explicit admitted ranges.
- No compatibility inference from plugin code behavior.
- No bypass of authority, policy, resource, provenance, or effect gates through conformance evidence.
