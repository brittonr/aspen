## Context

Aspen has several existing Nickel seams: `.aspen/ci.ncl`, `crates/aspen-ci/src/config/schema/ci_schema.ncl`, `crates/aspen-nickel/src/schema/node_config.ncl`, `schemas/deploy-protocol.ncl`, `test-harness/schema.ncl`, and `docs/crate-extraction/policy.ncl`. The next step is not simply "more Nickel"; it is a typed boundary policy that decides when Rust owns the data model and generates Nickel contracts, and when Nickel owns the configuration language consumed by Rust.

## Goals / Non-Goals

**Goals:**

- Make operator-facing config/evidence fail closed before runtime side effects.
- Prevent Rust/Nickel schema drift with generated artifacts and freshness checks.
- Keep Nickel contracts typed, documented, bounded, and queryable.
- Preserve Rust as the owner of behavior, cryptographic invariants, wire compatibility, and hot-path runtime logic.

**Non-Goals:**

- Do not move Raft state-machine logic or async runtime behavior into Nickel.
- Do not put secrets, raw tokens, private keys, or capability bearer values in Nickel.
- Do not replace Rust type checking with Nickel for protocol/wire compatibility.
- Do not require hand-written duplicate Nickel schemas when Rust already owns a serializable struct/enum.

## Decisions

### 1. Classify every Nickel candidate by source of truth

**Choice:** Use two source-of-truth classes.

- `rust-derived`: schema-bearing Rust structs/enums generate Nickel contracts.
- `nickel-authored`: human-facing modular config remains Nickel-first and exports validated JSON/TOML into Rust.

**Rationale:** receipts/protocol DTOs are already Rust structs and must not drift from serialization, while CI/node/profile configs benefit from Nickel's merge/default/documentation model.

### 2. Generate Nickel from Rust where Rust owns serialized shape

**Choice:** Rust DTOs that are serialized/persisted/operator-facing should derive schema metadata, then a generator emits Nickel contracts with enum choices, record contracts, defaults when known, docs when available, and bounded collection predicates where modeled.

**Implementation:** Start from `schemars`/Serde-visible shapes or a small Aspen-owned schema IR if direct JSON-schema conversion loses defaults/contracts. Check generated files into `schemas/` only with a freshness gate.

### 3. Keep Nickel-first surfaces modular and contract-rich

**Choice:** CI pipeline config, node/cluster profiles, feature bundles, trust/bootstrap policy, snix executor policy, and test/fault manifests should be Nickel-authored modules with typed contracts, defaults, and `not_exported` local/helper fields.

**Implementation:** Rust consumes only validated exported data. Human-facing composition stays in Nickel.

### 4. Add drift and negative evidence gates

**Choice:** Every contract family needs both positive export/typecheck tests and negative fixtures proving invalid config/evidence is rejected.

**Implementation:** Add a checker that regenerates Rust-derived contracts and fails on uncommitted diffs; run `nickel typecheck`/`nickel export`; run Rust round-trip/serde tests for the originating DTOs.

## Risks / Trade-offs

**Generated Nickel loses semantic contracts** → Mitigate with Aspen schema annotations or post-generation contract overlays for bounds, non-empty strings, max lengths, and secrecy policies.

**Nickel becomes a dumping ground for runtime behavior** → Mitigate with explicit non-goals and review gates: Nickel describes configuration/evidence shape, not distributed behavior.

**Two-way generation gets ambiguous** → Mitigate by allowing only one source of truth per family and documenting that classification in the contract registry.

**Secret material leaks through config examples** → Mitigate by requiring reference/path/handle validation and adding negative tests for raw bearer credential fields.
