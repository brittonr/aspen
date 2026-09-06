## ADDED Requirements

### Requirement: Exact producer inputs
r[molten.node_content.inputs] Validation MUST use the existing pinned source revisions without weakened hashes, feature-pruning substitutions, or replacement toolchains. Cache transport MUST preserve canonical source identity. Missing inputs or incompatible validation tools MUST remain explicit blockers.

#### Scenario: Exact private source is recovered
- GIVEN an owner repository contains the required Git commit
- WHEN the cache receives that commit
- THEN its Git and locked NAR identities match before compilation and consumer lockfiles remain unchanged.

### Requirement: Normal node lifecycle owns content service
r[molten.node_content.lifecycle] Protected content serving MUST compose with the normal node startup, capability-rooted state, service lifecycle, and stop path. The node MUST require explicit read grants before content exposure. Pins and transport locators MUST NOT create read authority.

#### Scenario: Node stops
- GIVEN a normal node with an active protected content service
- WHEN its normal lifecycle stops
- THEN its content router stops and the node records the bounded outcome without retaining an independent fixture server.

### Requirement: Real normal-service VM proof
r[molten.node_content.vm] Tests MUST use normal node and client commands in distinct VMs without shared payload mounts. They MUST cover exact transfer, server-side denial, wrong identity, clean storage restart, and damaged-store rejection. The retrieved archive MUST pass the unchanged Mantle/Onix native replay separately.

#### Scenario: Fresh client receives retained archive
- GIVEN a normal storage node restarts from its persistent VM disk without an archive seed
- WHEN a fresh authorized client retrieves the content
- THEN exact archive verification and the separate native replay pass.

### Requirement: Scope remains bounded
r[molten.node_content.scope] Evidence MUST remain VM-only and MUST NOT imply production readiness, package provenance from Molten identity, physical support, or default eligibility. The workflow MUST NOT use Stage0, rebuild Darkhttpd or Mantle, or install host blob services.

#### Scenario: Required gate is blocked
- GIVEN an unavailable or incompatible validation prerequisite
- WHEN closeout is requested
- THEN the change remains open without rewritten expectations or fabricated success evidence.
