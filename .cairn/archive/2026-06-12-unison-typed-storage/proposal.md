## Why

Molten will persist runtime metadata, receipts, indexes, actor/vat snapshots, and application values. If persisted data is identified only by ad hoc table names or JSON blobs, the runtime will lose type information and accumulate manual translation layers at every storage boundary.

Unison's typed durable storage is useful prior art: persisted values retain enough code/type identity to be loaded later without dependency ambiguity. Molten should adapt this by storing canonical Preserves values with schema/type/artifact refs, explicit capabilities, and upgrade recipes.

## What Changes

- Define typed durable references for persisted Molten values.
- Store values as canonical Preserves bytes or content refs, never raw Rust memory layouts.
- Bind stored values to schema/type artifact ids, producer artifact ids, storage namespace, policy refs, and evidence refs.
- Require load operations to validate schema/type compatibility and artifact availability before returning typed values.
- Represent migrations as content-addressed artifacts with source schema, target schema, executable transformer, policies, and receipts.
- Support Redb as the first local durable store and Iroh blobs for large immutable stored payloads.
- Keep storage effects behind declared effect manifests and admitted storage handlers.

## Impact

This gives Molten a principled path for durable metadata, receipts, snapshots, and application values. The first implementation can persist and load a small schema-tagged Preserves value through Redb, reject schema mismatches, and emit receipts for put/get operations.
