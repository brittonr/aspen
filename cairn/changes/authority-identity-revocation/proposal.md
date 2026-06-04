## Why

Molten's runtime model depends on capabilities, evidence, actor ids, node ids, artifact authorship, gatekeeper resolution, and policy admission. Those surfaces need one coherent authority and identity model. Without it, artifact installation, remote sync, effect handling, storage access, replay, catalog visibility, and upgrade sessions can each invent incompatible identity or revocation rules.

## What Changes

- Define principals, nodes, actors, services, sessions, artifacts, and handler profiles as explicitly typed identities.
- Bind identities to key material, UCAN/Basalt capabilities, attenuation, expiry, delegation chains, and evidence refs.
- Define actor and service ids as scoped identities, not ambient authority.
- Define revocation, expiry, key rotation, authority loss, and cleanup semantics.
- Require every trust-boundary action to present and record the authority context used for admission.
- Make gatekeeper resolution the audited path from long-lived credentials to live scoped references.
- Integrate revocation with dataspace assertion cleanup, effect handlers, storage access, remote sync, catalog visibility, and replay.

## Impact

This becomes Molten's security spine. The first milestone can model principal/node/actor ids, capability refs, delegation/attenuation metadata, and a revocation check used by local dataspace publish/observe and effect admission.
