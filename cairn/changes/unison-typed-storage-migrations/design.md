## Context

Molten already has typed storage requirements. This change focuses on migration and long-lived read semantics: persisted values must say what they are, what produced them, which contracts may consume them, and how they can evolve.

## Design

### Typed value record

A typed storage record binds:

- canonical value ref or content/chunk ref;
- value schema ref and schema identity mode;
- producing artifact ref and optional consumer artifact refs;
- storage handler/profile ref;
- policy, capability, retention, provenance, and evidence refs;
- migration lineage refs.

### Compatibility before read

When a caller expects schema E and a stored value has schema A, the pure core checks schema identity and admitted compatibility receipts. Unique schema mismatch denies unless an explicit alias or migration receipt is present. Structural compatibility must still satisfy policy and evidence requirements.

### Migration recipe gate

A migration recipe is an artifact with source schema, target schema, executable recipe ref, effect manifest, handler profile, policy refs, provenance/source-gate refs, test evidence, and rollback/lineage metadata. Applying a migration emits receipts for preflight, execution, output validation, and storage lineage.

### No serialized functions

Stored values may reference executable artifact refs and migration recipes, but they cannot store arbitrary functions, closures, or decoders as value identity. Decoders are admitted artifacts with normal provenance, effects, and policy checks.

### Functional core and shell

Pure cores validate typed record metadata, schema compatibility, migration plans, and read decisions. Shells read/write Redb/chunks, call execution adapters for migration recipes, persist receipts, and enforce capability/policy/retention/provenance gates.

### Non-goals

- Do not adopt Unison value serialization, runtime, hash format, or typechecker.
- Do not make typed storage refs authority to read, write, delete, or migrate data.
- Do not treat mutable names or decoder source text as storage type identity.
- Do not bypass retention, confidentiality, or provenance gates during migration.