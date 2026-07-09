## Context

Pluggable consensus engines are already represented by manifest algorithm profiles and the consensus engine registry. However, common manifest construction still defaults to Raft unless callers manually build a `ConsensusAlgorithmProfileInput`. That makes the engine choice feel like code wiring rather than a cluster configuration decision.

## Decisions

### Cluster config owns profile selection

**Choice:** Introduce a cluster consensus config value that names `algorithm_profile`, optional `profile_version`, optional `placement_ref`, and `required_evidence_refs`.

**Rationale:** This is the minimal cluster-facing input needed to select an engine while letting manifest construction derive read modes, quorum rule, membership policy refs, caveats, and default Raft evidence from existing validated builders.

### Omitted selection defaults to Raft

**Choice:** The config default is the current Raft production profile and version.

**Rationale:** Existing deployments and fixtures must keep their current safety semantics unless an operator explicitly selects another profile.

### Runtime admission remains fail-closed

**Choice:** Config selection only shapes the manifest. Runtime construction still resolves the selected profile through the consensus engine registry and production admission policy.

**Rationale:** A cluster can express that it needs a profile, but config alone cannot promote an experimental or evidence-incomplete engine into production.

## Validation strategy

Run focused consensus/config tests, `cargo fmt --check`, and Cairn validation for the lifecycle package.
