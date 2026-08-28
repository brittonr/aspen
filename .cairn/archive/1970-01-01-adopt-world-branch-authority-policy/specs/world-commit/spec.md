# Molten World Commit Specification Delta

## Purpose

Adopt portable branch-authority policy while keeping current capability derivation, transfer, and enforcement in Molten.

## ADDED Requirements

### Requirement: Molten consumes branch policy without transferring runtime authority

r[molten.world_branch_authority.adoption] Molten MUST consume one reviewed immutable Basalt world-branch authority policy cohort. Basalt decisions MUST remain supplied policy facts and MUST NOT mint, move, store, or enforce capabilities.

#### Scenario: Basalt admits a normalized branch action

- GIVEN the policy identity, normalized capability facts, and authority inputs are current
- WHEN Basalt returns an admitted branch mode
- THEN Molten MAY build a realization plan under its own runtime gates

#### Scenario: Policy receipt is presented as a capability

- GIVEN a passing Basalt receipt exists without current realization evidence
- WHEN branch activation runs
- THEN Molten MUST deny activation

### Requirement: Branch authority uses explicit derivation obligations

r[molten.world_branch_authority.derivation] Molten MUST support closed copyable, attenuated, linear, simulation-only, promotion-gated, replace-before-activation, and non-branchable modes. Each admitted mode MUST produce explicit obligations, and unknown modes MUST deny.

#### Scenario: Attenuated capability is requested

- GIVEN policy permits attenuation and names required limits
- WHEN planning runs
- THEN the plan MUST require a verifiably narrower destination grant

#### Scenario: Unknown branch mode appears

- GIVEN a policy output contains an unsupported mode
- WHEN Molten validates the output
- THEN it MUST deny instead of treating the mode as copyable

### Requirement: Linear authority cannot remain active on both branches

r[molten.world_branch_authority.linear] Molten MUST realize linear movement through generation-fenced durable transfer. Destination activation MUST require evidence that the source no longer owns active use.

#### Scenario: Linear transfer commits

- GIVEN the source owns the exact current generation and every transfer gate passes
- WHEN the durable transfer commits
- THEN source use MUST become unavailable before destination activation succeeds

#### Scenario: Source still appears active

- GIVEN current observations cannot prove source deactivation
- WHEN destination activation runs
- THEN Molten MUST deny activation as ambiguous ownership

### Requirement: Simulation authority cannot reach live adapters

r[molten.world_branch_authority.simulation] Molten MUST bind simulation-only grants to exact deterministic simulation adapters. Missing or failed simulation support MUST deny instead of falling back to live effects.

#### Scenario: Simulation adapter is available

- GIVEN an exact admitted simulation profile implements the requested capability
- WHEN branch activation runs in simulation mode
- THEN Molten MAY bind only that simulation adapter

#### Scenario: Simulation adapter is missing

- GIVEN a live adapter exists but no admitted simulation adapter exists
- WHEN branch activation runs
- THEN Molten MUST deny without invoking the live adapter

### Requirement: Authority is rechecked at activation and promotion

r[molten.world_branch_authority.activation] Molten MUST recheck current policy, UCAN, revocation, replay, scope, durable ownership, adapter, and effect facts at branch activation and promotion.

#### Scenario: Stored observation became stale

- GIVEN the world commit references an earlier passing authority observation
- WHEN current revocation or policy facts differ
- THEN activation MUST use current facts and deny when they no longer admit use

#### Scenario: Promotion reservation is complete and committed

- GIVEN the exact promotion plan has one complete committed reservation set and a selected reservation for the candidate
- WHEN promotion-gated activation admission runs
- THEN Molten MAY admit branch activation without authorizing effect dispatch

#### Scenario: Promotion observation authorizes dispatch

- GIVEN a promotion observation claims dispatch authority or has an incomplete, uncommitted, or crossed reservation
- WHEN promotion-gated activation admission runs
- THEN Molten MUST deny before branch activation

### Requirement: Branch-authority evidence excludes bearer material

r[molten.world_branch_authority.evidence] Branch plans and receipts MUST exclude bearer tokens, private keys, credentials, secret entropy, raw capability paths, and private policy bodies. They MUST state that decisions and observations do not prove future enforcement.

#### Scenario: Receipt projection receives a bearer token

- GIVEN shell inputs include private capability material
- WHEN safe receipt projection runs
- THEN the output MUST omit the material and retain only approved identities and bounded decisions

### Requirement: Branch-authority verification covers widening and escape

r[molten.world_branch_authority.verification] Molten MUST test every supported mode and negative widening, duplicate linear use, stale currentness, simulation escape, promotion bypass, secret disclosure, and enforcement overclaims.

#### Scenario: Focused branch-authority rail runs

- GIVEN positive and negative fixtures use the reviewed Basalt cohort
- WHEN the focused verification rail runs
- THEN it MUST report supported modes and current-enforcement non-claims
