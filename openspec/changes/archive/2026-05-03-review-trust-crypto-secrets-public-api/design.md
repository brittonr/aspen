## Context

Trust/crypto/secrets is an aggregate family spanning pure helpers, portable type contracts, service libraries, and runtime compatibility consumers. Recent slices created clean portable islands, but the aggregate policy entry still models the family as one pseudo-candidate and the checker warns that direct dependency checks are deferred because no package maps to that pseudo-candidate.

## Goals

- Identify canonical reusable public API surfaces.
- Identify runtime or compatibility surfaces that must not be marketed as reusable extraction targets yet.
- Record blockers before promoting the family to `extraction-ready-in-workspace`.
- Capture durable evidence for future promotion work.

## Non-Goals

- Do not promote readiness in this review unless fresh evidence proves real crate-level checker coverage.
- Do not split additional runtime storage, trust async, or secrets implementation features in this change.
- Do not change Rust APIs.

## Decisions

### 1. Treat portable sub-surfaces as the reusable API core

**Choice:** `aspen-crypto` default/no-default helpers and `aspen-secrets-core` are the clearest reusable public surfaces. Selected `aspen-trust` pure helpers remain candidate reusable APIs, but their default async/Tokio service surfaces need explicit policy before family promotion.

**Rationale:** Fresh dependency inspection shows `aspen-crypto --no-default-features` is transport-free and `aspen-secrets-core --no-default-features` depends only on serialization support.

### 2. Keep runtime-heavy secrets and handler crates out of the reusable set

**Choice:** `aspen-secrets` remains a service/runtime implementation crate for now, and `aspen-secrets-handler` remains a compatibility/runtime consumer even though its default factory/runtime-context boundary is now explicit.

**Rationale:** `aspen-secrets --no-default-features` still intentionally pulls implementation dependencies such as Tokio, age, PKI/X.509 crates, `iroh-base`, and crypto execution crates. `aspen-secrets-handler` owns RPC compatibility around client API messages, not a portable standalone API.

### 3. Defer readiness promotion until checker coverage is real

**Choice:** Keep `trust-crypto-secrets` at `workspace-internal` in this change.

**Rationale:** The current checker maps the family to a pseudo-candidate and emits a warning that direct package dependency checks are deferred. Promoting while the checker cannot validate the real crates would overstate evidence.

## Risks / Trade-offs

- **False confidence from aggregate evidence** → Mitigate by recording the checker warning and requiring crate-level candidate mapping before promotion.
- **Over-promising serde stability** → Mitigate by documenting a serialization-contract blocker for trust share/envelope/state types and secrets-core persisted state vs request DTOs.
- **Async/Tokio ambiguity in `aspen-trust`** → Mitigate by requiring an explicit decision: either document async service dependencies as acceptable, or feature-gate/move async manager and reencryption APIs.
