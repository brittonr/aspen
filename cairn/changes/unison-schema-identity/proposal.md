## Why

Molten will compare schemas for envelopes, storage records, protocol payloads, effect manifests, capabilities, and policy inputs. If every schema is treated as purely nominal, equivalent generic shapes cannot be reused. If every schema is treated as purely structural, domain-specific values with the same shape can be confused.

Unison's unique vs structural type distinction is useful prior art. Molten should adapt it for Preserves/schema artifacts: some schemas are equivalent by canonical structure, while others are equivalent only through explicit declared identity.

## What Changes

- Add schema identity modes to Molten schema artifacts: structural, unique/nominal, and explicitly branded structural variants if needed.
- Define canonical structural hashes over normalized schema shapes, independent of mutable names and docs.
- Define unique schema ids over declared schema artifact identity and optional brand metadata.
- Require storage, choreography payloads, effect manifests, capabilities, and policy contracts to state whether structural or unique compatibility is expected.
- Add compatibility checks that can distinguish shape equivalence, nominal identity, admitted aliases, and migration-required mismatches.
- Record schema identity decisions in receipts for writes, loads, protocol installation, effect binding, and policy admission.

## Impact

This prevents confusing structurally identical but semantically different values, while still allowing reusable generic schemas where structural equivalence is intended. The first milestone can implement structural hashes for normalized Preserves schemas and a unique schema artifact id mode, then reject storage/protocol mismatches unless the expected identity mode admits them.
