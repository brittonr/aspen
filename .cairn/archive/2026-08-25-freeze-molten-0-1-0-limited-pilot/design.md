# Design

## Context

A release candidate must remain immutable while release metadata and lifecycle receipts are added. Building from the release-documentation branch would evaluate changed source and would not prove the frozen candidate.

## Candidate Identity

The candidate source is the clean detached Git commit `a4f111690b6962f04d9320fd93d09c7dd1ad2fd0` with tree `58a6763c3668121ffa7309195f8d2c76ef4950d3`.

The domain-separated frame is:

```text
<molten-source-candidate-v1 "a4f111690b6962f04d9320fd93d09c7dd1ad2fd0" "58a6763c3668121ffa7309195f8d2c76ef4950d3">
```

BLAKE3 over those exact UTF-8 bytes, without a trailing newline, is `80e3ceb18784504c7573191fce72e121d0789613c6c5f7bdcecbdd9ae0e4cdb7`.

This identity binds the Git commit and tree. It does not bind external dependencies, build outputs, runtime state, or release authority. Separate lock, provenance, build, source-gate, and candidate receipts cover those claims.

## Execution Boundary

All candidate builds run in a clean detached worktree at the frozen commit. The release branch owns lifecycle files, reviewed evidence copies, release notes, and publication metadata. No candidate build consumes release-branch modifications.

## Evidence Families

Three candidate identity mechanisms were checked:

1. Git commit and tree in a domain-separated BLAKE3 frame.
2. A Nix store source path identity.
3. A sorted per-file BLAKE3 manifest.

The Git frame is selected because it is portable, immutable, reviewable, and independent from one Nix source filter. The Nix path remains build evidence, not source identity. A per-file manifest would be valid only after defining and testing a second canonicalization contract.

## Invariants

- Every release artifact identifies the same candidate source reference.
- Candidate checks run from a clean detached checkout of the frozen commit.
- Passing fixtures do not count as release evidence.
- Warning-only Octet output does not count as strict source-gate acceptance.
- Configuration-current warning-only Octet evidence can support only the caveated limited pilot.
- A failed, skipped, unavailable, stale, or mismatched artifact denies publication unless the accepted pilot policy explicitly requires that deny evidence.
- The pilot decision names allowed workloads, exclusions, rollback triggers, stop conditions, and caveats.
- The release tag points to the frozen candidate, not the later evidence-documentation commit.

## Adversarial Audit

Audit checks include dirty-worktree substitution, changed Git trees, mixed-source bindings, missing evidence categories, malformed references, stale README evidence, unsupported VM execution, and broad-production wording. The release remains blocked if any required executable evidence is missing.

## Boundaries and Non-Claims

The pilot does not establish real-WAN behavior, sustained SLOs, fleet pressure, adversarial security, production consensus, destructive-operation readiness, or general production eligibility. Evidence receipts record bounded observations and identity links only.
