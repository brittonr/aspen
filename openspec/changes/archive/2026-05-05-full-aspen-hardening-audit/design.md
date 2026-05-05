## Context

Aspen is intentionally self-hosting and production-oriented: the same platform carries source hosting, CI, Nix artifacts, cluster orchestration, federation, secrets, and operator evidence. The security posture depends less on one central gate and more on many small authority boundaries staying aligned: request classification must match handler behavior, tokens must be scoped and presenter-bound where intended, proxy paths must preserve or reject credentials correctly, reserved storage prefixes must not be reachable through generic KV operations, and evidence must be durable without becoming a credential sink.

Recent security slices hardened deploy status auth, federation operation scopes, proxy token preservation/audience checks, SNIX store capabilities, and reserved SNIX key prefixes. The full audit should build on those slices by turning the implicit checklist into a durable audit program.

## Goals / Non-Goals

**Goals:**

- Produce a complete, reproducible inventory of Aspen security authority boundaries.
- Classify each boundary by asset, actor, operation, expected authorization, persistence, logging/output surfaces, and verification evidence.
- Ensure every non-public request path has a fail-closed `Operation` mapping or an explicitly documented public exemption.
- Confirm credential material is never printed, persisted to receipt/log/error surfaces, or stored with weak file permissions.
- Prove high-risk bypass classes with negative tests or deterministic source-order/drift tests.
- Convert findings into prioritized OpenSpec children or direct small remediation commits.

**Non-Goals:**

- Do not implement all remediation in this umbrella change.
- Do not print, decode, or exfiltrate existing operator secrets/tokens/tickets.
- Do not replace Tiger Style, fuzzing, madsim, or dogfood; compose them as evidence sources.
- Do not weaken existing Iroh-only or Raft-backed design constraints to make auditing easier.

## Decisions

### 1. Inventory-first audit

**Choice:** The first implementation slice will create an audit inventory artifact before changing runtime behavior.

**Rationale:** Aspen's risk comes from cross-crate drift. An inventory lets later slices prove completeness instead of rediscovering variants, prefixes, tokens, and handler paths.

**Alternative:** Start by hand-auditing the highest-risk crate. Rejected because it creates good local fixes without proving full-system coverage.

**Implementation:** Generate or maintain a structured inventory under the active change evidence directory first, then promote reusable scripts/docs once the schema stabilizes. Include source handles, crate/module owner, authority type, expected guard, and verification handle.

### 2. Authority matrix over prose-only review

**Choice:** Every protected operation must map to an explicit authorization expectation in an audit matrix.

**Rationale:** Security regressions in Aspen often arise when request enums, handlers, CI/job operations, or reserved storage prefixes evolve without updating the auth contract.

**Alternative:** Rely on code review and existing tests. Rejected because enum drift and feature-gated paths need deterministic coverage.

**Implementation:** Matrix rows include `ClientRpcRequest`, `CiRequest`, native/proxy handlers, SNIX store ops, Forge/federation ops, deploy/admin ops, jobs/CI execution ops, secrets/trust ops, persisted prefixes, and public exemptions.

### 3. Negative proof for every high-risk boundary

**Choice:** The audit accepts a boundary only when there is negative evidence that the obvious bypass fails.

**Rationale:** A positive test that authorized credentials work does not prove generic or stale credentials fail.

**Alternative:** Require only happy-path integration tests. Rejected because the known hardening seams are bypass-oriented.

**Implementation:** Use unit tests, source-order tests, drift tests, reserved-prefix fixtures, no-mutation tests, and dogfood receipts. Examples: generic KV caps do not authorize SNIX store mutation; Forge write caps do not authorize federation pull/push; key-bound tokens are rejected before proxy forwarding; deploy status requires cluster admin; condition keys in conditional batches are included.

### 4. Redaction as an evidence gate

**Choice:** Audit evidence must prove operator-facing outputs are credential-safe.

**Rationale:** Aspen receipts and diagnostics are meant to be shared and retained. They must not include token/ticket prefixes, bearer strings, private keys, connection strings, or secret payloads.

**Alternative:** Rely on developer discipline. Rejected because receipts/logs/errors are broad and durable.

**Implementation:** Add redaction grep/checker fixtures against representative CLI output, error formatting, receipt JSON, and logs. The checker should use synthetic secret-like fixtures and must not inspect real secrets.

### 5. Remediation as child slices

**Choice:** Findings become focused OpenSpec children or direct small commits, not one giant audit patch.

**Rationale:** Aspen's preferred workflow is small verified hardening slices with clear proof.

**Alternative:** Batch all fixes into one mega-change. Rejected because it obscures review and verification causality.

**Implementation:** Tasks require a finding triage table with severity, owner, affected requirement IDs, intended verification, and chosen follow-up change/commit.

## Audit Domains

1. **Auth and request classification:** `AuthenticatedRequest`, `ClientRpcRequest::to_operation`, `CiRequest`, batch operations, public exemptions, auth-before-handler dispatch, presenter binding.
2. **Proxy and federation:** token preservation, proxy token eligibility, federation pull/push scopes, credential lookup/storage, remote handshake behavior, federation CLI outputs.
3. **Storage and reserved prefixes:** generic KV versus domain-specific SNIX/secrets/trust/forge/admin prefixes, scan/watch/batch/conditional behavior, state-machine mutation paths.
4. **Secrets, tokens, and tickets:** token issuance/delegation, root/bootstrap semantics, audience binding, ticket serialization, file permissions, redaction, revocation, rotation.
5. **Jobs, CI, deploy, and dogfood:** execution sandboxing, artifact and receipt schemas, deploy admin gates, job log access, command allowlists, failure evidence.
6. **Network and transport:** Iroh ALPN routing, no HTTP control-plane regressions, snapshot/blob transfer assumptions, rate limiting, connection metrics without credential leakage.
7. **Nix/SNIX/build supply chain:** native build sandboxing, cache gateway boundaries, nar bridge exposure, fallback subprocess gates, artifact identity and verification.
8. **Dependencies and feature gates:** feature-gated public API authority surfaces, default/no-default graphs, vendored dependencies, unsafe/public unsafe API inventory.

## Risks / Trade-offs

- **Audit scope creep:** Mitigate by treating this as an umbrella and cutting child remediation changes.
- **False confidence from static checks:** Mitigate by pairing static inventory with targeted negative runtime tests for high-risk paths.
- **Secret exposure during audit:** Mitigate by using synthetic fixtures only and redacting all secret-like values in captured evidence.
- **Feature-gated blind spots:** Mitigate by recording the exact feature set for each evidence command and requiring separate coverage for forge, federation, jobs, snix, secrets, and testing gates.
