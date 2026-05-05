## Phase 1: Specification Foundation

- [x] Create the full hardening audit proposal, design, task plan, and delta spec.
- [x] Validate the OpenSpec change and record expected helper warning: 21 implementation tasks remain open.

## Phase 2: Inventory and Threat Model

- [x] Build a structured audit inventory covering request enums, handlers, persisted prefixes, token/ticket files, CLI outputs, receipts, logs, external process boundaries, feature gates, and existing tests. Evidence: `evidence/audit-inventory.md`, `evidence/audit-inventory.json`.
- [x] Write a threat model that maps assets, actors, trust boundaries, abuse cases, and expected controls for every audit domain. Evidence: `evidence/threat-model.md`.
- [x] Record which surfaces are intentionally public and which are protected by capability, presenter binding, cluster admin, federation proxy delegation, SNIX-specific caps, or filesystem permissions. Evidence: `evidence/authority-boundary-classification.md`.

## Phase 3: Authorization and Authority Boundary Audit

- [x] Verify every non-public `ClientRpcRequest` and `CiRequest` variant classifies to the intended `Operation` or fails closed. Evidence: `evidence/authorization-matrix.md`, `evidence/authorization-matrix.json`; fixed `CiRequest::CiGetRefStatus` drift and raised the `ClientRpcRequest` classification drift-test bound.
- [x] Verify auth-before-dispatch and presenter binding across direct, native, blob, proxy, and feature-gated handler paths. Evidence: `evidence/auth-before-dispatch.md`, `evidence/auth-before-dispatch.json`; added regression coverage for blob-before-dispatch ordering, local-handler-before-proxy fallback, and verified-token proxy handoff.
- [x] Verify reserved storage prefixes cannot be disclosed or mutated through generic KV read/write/scan/watch/batch/conditional operations. Evidence: `evidence/reserved-prefix-audit.md`, `evidence/reserved-prefix-audit.json`; generic KV exact, scan/watch, batch, and conditional-batch condition-key paths now fail closed to cluster-admin for internal prefixes.
- [ ] Verify SNIX, Forge, federation, deploy, jobs/CI, secrets/trust, and admin operations use domain-specific capabilities rather than generic prefixes.
  - Secrets/trust high-risk slice captured in `evidence/domain-capability-audit.md`, `evidence/domain-capability-audit.json`; secrets, transit, PKI, and Nix-cache signing requests now use domain-specific operations instead of generic `_secrets:` data prefixes.
  - Net service-mesh slice captured in `evidence/domain-capability-audit.md`, `evidence/domain-capability-audit.json`; publish/unpublish/lookup/list now require net-domain capabilities instead of generic `/_sys/net/svc/` data prefixes.
  - CI/jobs slice captured in `evidence/domain-capability-audit.md`, `evidence/domain-capability-audit.json`; CI pipeline and job/worker requests now require `CiRead`/`CiWrite`/`JobsRead`/`JobsWrite` instead of generic `_ci:`, `_jobs:`, or `__worker:` data prefixes.
  - Blob/docs/hooks slice captured in `evidence/domain-capability-audit.md`, `evidence/domain-capability-audit.json`; blob, docs, and hooks requests now require `BlobRead`/`BlobWrite`/`DocsRead`/`DocsWrite`/`HooksRead`/`HooksWrite` instead of generic `_blob:`, `_docs:`, or `_hooks:` data prefixes.
  - Coordination/lease slice captured in `evidence/domain-capability-audit.md`, `evidence/domain-capability-audit.json`; locks, counters, sequences, rate limiters, barriers, semaphores, RW locks, queues, service registry, and leases now require `CoordinationRead`/`CoordinationWrite` instead of generic coordination storage prefixes.
  - Observability slice captured in `evidence/domain-capability-audit.md`, `evidence/domain-capability-audit.json`; traces, metrics, and alerts now require `ObservabilityRead`/`ObservabilityWrite` instead of generic `_sys:traces:`, `_sys:metrics:`, or `_sys:alerts:` data prefixes. Remaining generic internal-domain mappings stay open for follow-up slices.
- [ ] Add or update negative drift tests for every high-risk bypass class discovered by the matrix.

## Phase 4: Credential, Redaction, and Evidence Audit

- [ ] Audit token/ticket generation, delegation, persistence, file permissions, revocation/rotation, and root/bootstrap semantics.
- [ ] Audit CLI output, logs, errors, debug formatting, receipts, test fixtures, and docs for credential leakage using synthetic secret-like fixtures only.
- [ ] Verify dogfood, CI, deploy, and diagnostic receipts identify artifacts/runs without log scraping or credential values.
- [ ] Add redaction regressions for any output surface that handles token/ticket/secret-like material.

## Phase 5: Execution, Network, and Supply-Chain Audit

- [ ] Audit jobs, CI executors, native build paths, SNIX build/eval/cache paths, and fallback subprocess gates for sandbox and command-boundary enforcement.
- [ ] Audit Iroh ALPN routing, proxy/federation handshakes, blob/snapshot transfer assumptions, rate limiting, and metrics for security-sensitive drift.
- [ ] Audit Nix flake inputs, vendored dependencies, unsafe/public unsafe surfaces, feature-gated dependency graphs, and release/build artifact identity.
- [ ] Add focused verification commands or fixtures for any missing sandbox, transport, or supply-chain guarantee.

## Phase 6: Findings, Remediation, and Acceptance

- [ ] Publish an audit report with severity, affected requirements, source handles, evidence handles, remediation owner, and follow-up change/commit for each finding.
- [ ] Split high-risk findings into focused OpenSpec child changes or direct verified remediation commits.
- [ ] Run the accepted audit gate set: strict OpenSpec validation, `git diff --check`, targeted cargo tests/checks, Tiger Style, and any new audit scripts.
- [ ] Archive this umbrella only after the inventory, report, required negative evidence, and high-risk remediation plan are complete.
