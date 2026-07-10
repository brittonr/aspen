## Context

Molten's evidence-first CLI style intentionally requires explicit refs at trust boundaries. That is safe but verbose. The same operator often supplies identical policy/resource/authority/evidence bundles to related commands, and today there is no reviewed artifact that names those bundles for reuse.

## Decisions

### Context profiles expand to existing command inputs

**Choice:** A context profile is not a new authority channel. It expands into the same explicit refs accepted by existing command cores.

**Rationale:** Existing subsystem gates already understand those refs. The profile only improves repeatability and reviewability.

### Profile expansion is pure and operation-scoped

**Choice:** Add a pure expansion core that takes a context profile, requested operation kind, command-declared requirements, and explicit overrides, then returns expanded refs or denial diagnostics.

**Rationale:** Context selection should be testable without filesystem reads, environment variables, network, or command execution.

### Receipts bind profile and expanded refs

**Choice:** Receipts for commands that use a profile record the profile ref and the expanded refs actually used. Downstream gates continue to evaluate the expanded refs.

**Rationale:** Reviewers need to know which profile was selected, but the canonical admission input remains the explicit refs.

### Profiles can constrain scope

**Choice:** Context profiles may include allowed operations, resources, topics, nodes, retention classes, visibility classes, and profile-tier caveats. Expansion denies if a command asks for a scope outside the profile.

**Rationale:** A reusable profile should reduce mistakes and prevent accidental overuse beyond reviewed context.

## Validation strategy

- Pure positive tests for profile expansion into policy/resource/authority/evidence refs.
- Negative tests for stale refs, unsupported operation scopes, missing required refs, contradictory override values, and profile-only authority attempts.
- CLI fixture tests on representative workflows without broad rewrites.

## Non-claims

Context profiles do not certify that refs are current, grant permissions, prove policy admission, or make transport/source/provenance evidence sufficient. They only package reviewed inputs for repeatable expansion.
