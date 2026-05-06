## ADDED Requirements

### Requirement: Cargo audit advisory remediation [r[aspen-hardening-audit.cargo-audit-remediation]]

Aspen MUST keep the dependency-security audit gate actionable by remediating newly reported vulnerabilities through dependency updates, feature pruning, or advisory-specific documented exceptions.

#### Scenario: Initial advisory inventory is reproducible [r[aspen-hardening-audit.cargo-audit-remediation.initial-inventory]]

- GIVEN `nix build .#checks.x86_64-linux.audit --no-link -L` fails
- WHEN remediation starts
- THEN the change MUST capture a reproducible advisory inventory that names each current RUSTSEC advisory, crate, version, dependency path or owning surface, and initial disposition

#### Scenario: Wasmtime cluster is remediated before lower-risk clusters [r[aspen-hardening-audit.cargo-audit-remediation.wasmtime-first]]

- GIVEN the advisory inventory includes `wasmtime` sandbox or guest-runtime advisories
- WHEN remediation tasks are ordered
- THEN the Wasmtime/plugin-runtime dependency cluster MUST be updated, pruned, or explicitly fail-closed before TLS, archive, or warning-only clusters are marked complete
- AND affected plugin/runtime tests or checks MUST be run before the task is accepted

#### Scenario: TLS validation advisories remain security blockers [r[aspen-hardening-audit.cargo-audit-remediation.tls-blockers]]

- GIVEN the advisory inventory includes TLS, certificate-validation, CRL, or name-constraint advisories
- WHEN audit remediation is evaluated
- THEN those advisories MUST be fixed by dependency updates or path removal unless a checked-in exception documents the exact non-exposure and compensating control
- AND transport or upstream-cache callers affected by the dependency path MUST be checked

#### Scenario: Archive extraction advisories are tied to materialization checks [r[aspen-hardening-audit.cargo-audit-remediation.archive-materialization]]

- GIVEN the advisory inventory includes tar, PAX, symlink, chmod, or archive extraction advisories
- WHEN those dependencies are updated, pruned, or exceptioned
- THEN the remediation MUST include materializer, snix, or build-input verification that covers the affected extraction path or proves the path is unreachable

#### Scenario: Audit exceptions are narrow and reviewable [r[aspen-hardening-audit.cargo-audit-remediation.exception-policy]]

- GIVEN a vulnerability or warning remains after feasible dependency updates
- WHEN the audit gate is made green
- THEN any new ignore or allowance MUST name the advisory ID, crate, version/path, exposure rationale, compensating control, and removal trigger
- AND broad wildcard ignores or undocumented suppressions MUST NOT be accepted

#### Scenario: Final audit gate is green [r[aspen-hardening-audit.cargo-audit-remediation.final-green]]

- GIVEN all remediation and exception tasks are complete
- WHEN `nix build .#checks.x86_64-linux.audit --no-link -L` runs
- THEN it MUST pass without reporting unclassified vulnerabilities
- AND the worktree MUST remain clean after verification artifacts are staged or committed
