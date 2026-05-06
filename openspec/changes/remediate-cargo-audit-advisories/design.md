## Context

A fresh audit run failed at the Nix `audit` check. The direct JSON cargo-audit probe found 23 listed vulnerabilities, while the Nix check reported 22 after its advisory-db patching/ignore behavior. The already configured ignore is `RUSTSEC-2023-0071` for the `rsa` timing side-channel transitive dependency via `ssh-key` / forge. The new remediation should not paper over newly reported advisories by adding blanket ignores.

Current clusters observed from `cargo audit -n --ignore RUSTSEC-2023-0071 --json`:

- `wasmtime 36.0.3`: `RUSTSEC-2026-0006`, `RUSTSEC-2026-0020`, `RUSTSEC-2026-0021`, `RUSTSEC-2026-0085`, `RUSTSEC-2026-0086`, `RUSTSEC-2026-0087`, `RUSTSEC-2026-0088`, `RUSTSEC-2026-0089`, `RUSTSEC-2026-0091`, `RUSTSEC-2026-0092`, `RUSTSEC-2026-0093`, `RUSTSEC-2026-0094`, `RUSTSEC-2026-0095`, `RUSTSEC-2026-0096`.
- `rustls-webpki 0.103.9`: `RUSTSEC-2026-0049`, `RUSTSEC-2026-0098`, `RUSTSEC-2026-0099`, `RUSTSEC-2026-0104`.
- `aws-lc-sys 0.38.0`: `RUSTSEC-2026-0044`, `RUSTSEC-2026-0048`.
- `tar 0.4.44`: `RUSTSEC-2026-0067`, `RUSTSEC-2026-0068`.
- `astral-tokio-tar 0.5.6`: `RUSTSEC-2026-0066`.
- warning debt includes unmaintained and unsound advisories, including `vmm-sys-util 0.11.2` via `fuse-backend-rs` / snix-castore.

## Goals / Non-Goals

**Goals:**

- Return `nix build .#checks.x86_64-linux.audit --no-link -L` to green.
- Prefer dependency upgrades or feature pruning over ignores.
- Preserve Aspen's existing audit policy transparency by documenting every remaining allowed advisory.
- Validate affected Aspen behavior with focused tests/checks rather than only lockfile churn.

**Non-Goals:**

- No broad acceptance of sandbox, TLS, or archive vulnerabilities as ordinary warning debt.
- No architecture rewrite of Wasmtime/Hyperlight/plugin execution in this change unless a dependency upgrade exposes a minimal required API adaptation.
- No real secret material in audit evidence.
- No unrelated dependency modernization beyond what is required to clear or classify the current audit findings.

## Decisions

### 1. Remediate by risk cluster order

**Choice:** Drain clusters in this order: Wasmtime/plugin runtime, TLS/certificate validation, tar/archive extraction, warning classification.

**Rationale:** Wasmtime advisories affect the most explicit sandbox/guest-code boundary. TLS advisories affect remote trust decisions. Tar/archive advisories affect filesystem materialization and build inputs. Warning classification should happen after vulnerability remediation so it does not mask blocker work.

**Alternative:** Update everything in one broad dependency sweep. Rejected because it would make regressions harder to attribute and could hide dependency-edge changes behind large lockfile churn.

### 2. Ignores require advisory-specific rationale

**Choice:** New `cargo audit --ignore` entries MUST be advisory-specific and paired with a checked-in rationale that names the dependency path, exposure assessment, compensating control, and follow-up/removal trigger.

**Rationale:** The current audit failure is valuable signal. Passing the gate by suppressing clusters without rationale would weaken the supply-chain boundary.

**Alternative:** Add temporary ignores for all current advisories. Rejected because it would make `audit` green while leaving no enforceable remediation path.

### 3. Lockfile updates must be paired with surface checks

**Choice:** Each remediated cluster MUST run targeted Aspen checks for the touched surface before marking implementation complete.

**Rationale:** Dependency updates can change APIs and runtime behavior. The audit gate only proves advisories are no longer reported; it does not prove plugin/TLS/archive behavior still works.

## Validation Plan

- Capture an initial advisory inventory under the change evidence directory.
- After each cluster, run `cargo audit -n --ignore RUSTSEC-2023-0071` to show remaining advisories shrink or are classified.
- Run `nix build .#checks.x86_64-linux.audit --no-link -L` as the final supply-chain gate.
- Run targeted tests/checks for crates affected by Wasmtime/plugin, TLS/transport/cache, and archive/snix/materialization changes.
- Run strict OpenSpec validation and `git diff --check` before every commit.

## Risks / Trade-offs

- **Dependency graph churn:** Upgrading Wasmtime or TLS crates can force broad transitive updates. Mitigate with one cluster per commit and focused checks.
- **Vendored/forked dependencies:** Some advisories may be inherited through snix or vendored stacks. Mitigate by identifying the exact dependency path before patching.
- **False green from ignores:** Mitigate with advisory-specific rationale and a final inventory that distinguishes fixed, pruned, and allowed findings.
