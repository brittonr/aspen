## Context

`molten_core::startup_binding::admit` accepted only `molten-nix-source-binding-v1`. The closure validator already supports a role-complete graph set; it does not itself decide whether those graphs represent a full build or a runtime cohort.

## Decisions

### 1. A distinct producer profile, not a relaxed full profile

**Choice:** Add `molten-nix-guest-runtime-binding-v1` and accept it only as an exact string. The full profile remains unchanged.

**Rationale:** A boolean or schema change would silently admit weaker provenance under the existing name.

### 2. Runtime cohort is measured, not fabricated

**Choice:** The producer builds `build-closure` and all six graph files from the resolved runtime library closure and real BLAKE3 NAR identities. It emits an input-addressed Nix output and signs it with the operator key.

**Rationale:** Admission still measures every path it claims. The only narrowing is the declared cohort.

### 3. Guest trust remains a separate operator decision

**Choice:** The guest trust record selects the new output and its binding. No existing full-output trust is rewritten.

**Rationale:** Authority to run a guest runtime profile is explicit and revocable.

### 4. A node-root cache does not confer source authority

**Choice:** Verify the selected signature, executing bytes, and retained gate on every guest admission. Reuse only the existing closure measurement cache after authentication, under its existing immutable-store assumption.

**Rationale:** File path and length are not evidence of the executing bytes or of current successful authentication. A faster run under that earlier experiment is not qualification of the hardened path.
