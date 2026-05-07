## ADDED Requirements

### Requirement: Sibling UCAN source of truth [r[ucan-auth-integration.sibling-source-of-truth]]
Aspen auth MUST use the sibling `../ucan` implementation as the source of truth for generic UCAN issuance, parsing, verification, proof-chain, expiration, and attenuation semantics once this change is implemented.

#### Scenario: UCAN-backed issuance and verification [r[ucan-auth-integration.sibling-source-of-truth.ucan-backed-issue-verify]]
- GIVEN Aspen issues or verifies a capability token through the accepted auth API
- WHEN the token operation uses generic UCAN semantics such as signature verification, audience, expiration, facts, proof-chain links, or attenuation
- THEN the implementation SHALL route that semantic decision through `../ucan` or `../ucan/crates/ucan-core` public APIs rather than duplicating equivalent logic in Aspen

#### Scenario: Duplicate generic token kernels are rejected [r[ucan-auth-integration.sibling-source-of-truth.no-duplicate-kernel]]
- GIVEN an Aspen auth module contains token parsing, proof-chain verification, attenuation, or expiration logic that duplicates UCAN behavior
- WHEN the UCAN-backed adapter can express the same semantic decision
- THEN the duplicate Aspen logic SHALL be removed, replaced by a UCAN call, or documented as an Aspen-specific compatibility shim with focused tests

### Requirement: Adapter preserves Aspen authorization boundary [r[ucan-auth-integration.adapter-preserves-aspen-boundary]]
Aspen MUST keep project-specific capability vocabulary, operation checks, CLI/RPC behavior, and redacted receipts behind an explicit adapter while delegating generic UCAN token mechanics to the sibling crate.

#### Scenario: Aspen capabilities map to UCAN abilities [r[ucan-auth-integration.adapter-preserves-aspen-boundary.capability-mapping]]
- GIVEN Aspen capabilities for KV, Forge, CI, snix, federation, trust/secrets, runtime services, or cluster administration are represented in UCAN-backed tokens
- WHEN the adapter translates between Aspen capability values and UCAN abilities/resources
- THEN the mapping SHALL be documented, tested for attenuation, and fail closed for unsupported or unknown capabilities

#### Scenario: Operator-facing compatibility is preserved or receipted [r[ucan-auth-integration.adapter-preserves-aspen-boundary.operator-compatibility]]
- GIVEN an operator uses `aspen-token`, client RPC auth, federation credentials, or runtime service capability bindings
- WHEN Aspen switches verification to the UCAN-backed adapter
- THEN existing documented token commands and receipt redaction behavior SHALL remain stable unless an intentional migration receipt identifies the old behavior, the new behavior, and the compatibility risk

#### Scenario: Negative authorization cases fail closed [r[ucan-auth-integration.adapter-preserves-aspen-boundary.fail-closed-negative-cases]]
- GIVEN a token has capability escalation, an expired proof, a malformed proof link, a wrong audience, a replay/revocation denial, or a capability mapping that Aspen does not support
- WHEN the UCAN-backed adapter verifies or authorizes the token
- THEN Aspen SHALL deny the operation with bounded diagnostics and SHALL NOT expose raw tokens, private keys, cluster cookies, or secret material in receipts

### Requirement: Sibling dependency boundary is evidenced [r[ucan-auth-integration.dependency-boundary-evidenced]]
Aspen MUST prove the `../ucan` dependency is reproducible for the intended development and verification context and does not leak runtime-only dependencies into protected portable graphs.

#### Scenario: Relative sibling wiring is explicit [r[ucan-auth-integration.dependency-boundary-evidenced.relative-wiring]]
- GIVEN Aspen depends on the sibling `../ucan` repository during implementation
- WHEN Cargo, Nix, or CI resolves the dependency
- THEN the dependency wiring SHALL document whether `../ucan` is a local development path, a pinned fallback, or a vendored release input, and SHALL fail with an actionable diagnostic if the required source is unavailable

#### Scenario: Portable auth remains alloc-safe [r[ucan-auth-integration.dependency-boundary-evidenced.alloc-safe-core]]
- GIVEN portable Aspen crates depend on `aspen-auth-core` or UCAN core functionality
- WHEN dependency-tree checks run with protected no-default/alloc-only feature sets
- THEN `std`, filesystem, signer storage, runtime revocation stores, node bootstrap, RPC handlers, and root Aspen app crates SHALL remain outside those portable graphs

#### Scenario: Runtime shell owns impure integration [r[ucan-auth-integration.dependency-boundary-evidenced.runtime-shell-owns-impure-integration]]
- GIVEN UCAN integration needs signer storage, filesystem configuration, revocation adapters, or runtime clocks
- WHEN Aspen wires those effects
- THEN those dependencies SHALL live in `aspen-auth` or another runtime shell and SHALL NOT be required by `aspen-auth-core` default portable consumers
