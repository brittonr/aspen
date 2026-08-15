## Context

Local persistence is used by many subsystems, but those subsystems should not all own Redb transaction details. A store-port boundary lets pure cores describe what they need while a shell adapter performs database work after admission.

## Design

### Store port model

The first port should represent deterministic operations such as read artifact index, read delivery key, write cache entry, write raft snapshot metadata, list known refs, and apply atomic update. The port may be traits or plan/result records, but pure domain logic must be callable without Redb.

### Redb adapter shell

The Redb adapter owns database open/create, table definitions, read/write transaction lifetimes, migration checks, and low-level error mapping. It returns typed store results and canonical diagnostics to domain shells.

### Admission before writes

Domain planners should decide whether a request is admitted before any write transaction begins. Denied requests return evidence or diagnostics with an empty mutation plan.

### Migration order

Start with one high-value domain, preferably chunk store or artifact registry, and keep root-crate compatibility exports while callers migrate.

## Non-goals

- Do not replace Redb immediately.
- Do not change canonical refs, receipt schemas, or artifact bytes.
- Do not hide destructive or retention checks inside the store adapter.
