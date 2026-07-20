# Artifact-auth operational receipt

## ADDED Requirements

### Requirement: Non-production canary evidence is durably archived
r[molten.artifact_auth_operational_receipt.canary_archive] Molten MUST preserve its landed non-production artifact-auth canary as a self-contained public Cairn evidence bundle binding the exact product and artifact-auth revisions, harness source, operational receipt, successful fresh-process replay, capability-file generation rotation, stale-receipt denial, post-rotation status, typed manifest, and BLAKE3 inventory; the bundle MUST exclude private key material and mutable node state.

#### Scenario: Complete public canary bundle validates
GIVEN the exact public artifacts from the bounded Molten canary run
WHEN the manifest, inventory, receipt identity, positive replay, rotation denial, and reopened post-rotation state are checked
THEN every archived regular file is content-bound and the observed generation-1-to-2 transition remains reviewable without secret state

#### Scenario: Unsafe or incomplete archive fails closed
GIVEN a missing, drifted, symlinked, oversized, malformed, or secret-bearing candidate archive member
WHEN the canary archive is validated
THEN the evidence package is rejected before it can support a later admission review

### Requirement: Canary evidence does not grant authority
r[molten.artifact_auth_operational_receipt.canary_authority] Molten MUST label the archived canary as bounded non-production evidence and MUST keep `legacy_authoritative = true`, `standalone_authority_admitted = false`, and `rollback_available = true`; the archive MUST NOT claim network revocation freshness, membership, capability, federation, transport, storage, lifecycle, signing-policy, release authority, or production rollout.

#### Scenario: Passing canary remains observational
GIVEN a complete archived capture, replay, rotation-denial, and post-rotation status record
WHEN an operator reviews the canary result
THEN Molten's existing verification and admission decisions remain authoritative and a separate reviewed authority-admission change is still required
