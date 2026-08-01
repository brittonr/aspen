# Add collaboration-scope system extension

## Why

Molten separates fabric node membership from application authority, but it does not define collaboration scopes for people, agents, and shared resources. Downstream runtimes need current personal, conversation, group, and organization scope facts without turning node placement, transport identity, or UI state into authority.

A versioned system extension can own these distributed scope semantics. Basalt and UCAN remain the policy and capability boundaries, while resource content, credentials, memories, files, and product effects remain outside the extension.

## What Changes

- Add a versioned collaboration-scope system extension with canonical Preserves records for scopes, membership epochs, resource bindings, audience requests, admission snapshots, and receipts.
- Define pure transitions for scope creation, membership changes, resource binding, explicit resource moves, and snapshot admission.
- Require one current owner scope for each bound resource and deny implicit copies or moves between scopes.
- Compute effective audience admission from current membership, scope policy, resource policy, organization-floor policy, and caller-supplied Basalt and UCAN decision facts.
- Keep grants non-transitive. Reading or using a resource does not grant `resource/share` or permit a new scope binding.
- Host live mutations through admitted consistency, durable-state, logical-time, resource, authority, and evidence ports.
- Publish bounded metadata-only snapshots for downstream consumers without raw content, credentials, tokens, policy bodies, or identity documents.
- Add deterministic simulation and multiprocess fixtures for concurrent membership changes, stale snapshots, revocation, partitions, restart, and scope moves.

## Success Criteria

- Fabric membership and collaboration membership remain distinct types and stores.
- A stale membership epoch, scope revision, policy decision, or revocation fact cannot authorize a later scoped effect.
- Every proposed audience is admitted only when each selected subject passes the current effective scope policy.
- Narrower scopes cannot weaken an organization policy floor.
- A resource cannot be rebound, copied, or reshared through inherited read access.
- Animus, Lattice, and Tile can consume safe snapshots without becoming scope authorities.

## Non-Goals

- Human identity proofing, account storage, key discovery, global revocation freshness, or ambient trust.
- Storing raw memories, prompts, files, messages, credentials, tokens, or resource bytes.
- Making Molten fabric membership represent people, teams, tenants, or application audiences.
- Replacing Basalt policy evaluation, UCAN verification, product consent policy, or consumer effect authorization.
- Revoking information that a prior allowed effect already disclosed.
- Claiming whole-system confidentiality, exactly-once mutation, or release eligibility from extension receipts.

## Dependencies

- Existing Molten system-extension, consistency, durable-state, logical-time, resource, observability, and simulation boundaries.
- Nominal authority and artifact references from `type-authority-and-artifact-references`.
- An immutable reviewed Basalt and UCAN consumer cohort for live authority decisions.

## Reference Boundary

The public `yc-software/qm` collaboration and ACL design at commit `7f2c916360f1797a8ff2a77ce2ce40c5fabab087` is a requirements reference only. This change does not copy its TypeScript implementation, bearer-link model, database layout, credential handling, or security claims.

## Impact

- **Core:** pure collaboration-scope models, transitions, admission reducers, and receipt payloads.
- **Shell:** system-extension manifest, consistency and durable-state bindings, Basalt and UCAN decision inputs, logical-time currentness, and operator readback.
- **Schemas:** versioned Preserves scope, binding, snapshot, and receipt records.
- **Consumers:** metadata-only scope snapshots for Lattice, Animus, Tile, and other authorized products.
- **Testing:** positive and negative unit, property, simulation, restart, partition, multiprocess, privacy, authority, and replay fixtures.
