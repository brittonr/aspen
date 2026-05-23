# vmci-nix-store-boundary Specification

## Purpose

Defines VMCI Nix input and store-boundary requirements that keep large source materialization out of host-backed virtiofs paths and preserve bounded diagnostics for Nix store FD-pressure failures.

## Requirements
### Requirement: VMCI Nix Source Inputs Avoid Unbounded Virtiofs Traversal [r[vmci-nix-store-boundary.input-materialization]]

VMCI `ci_nix_build` execution MUST avoid strategies that force large public flake inputs or source closures to be copied, chmod-walked, or recursively traversed through host-backed virtiofs `/nix/store` paths during guest Nix evaluation/build.

#### Scenario: Public source input remains guest-local or cache-native [r[vmci-nix-store-boundary.input-materialization.public-input]]

- GIVEN a VMCI CI job evaluates a flake that depends on a large public input such as `nixpkgs`
- WHEN the guest prepares Nix inputs for `ci_nix_build`
- THEN the input MUST be resolved by a guest-local/cache-native strategy that does not require walking a host `/nix/store/...-source` tree through virtiofs
- AND the strategy MUST preserve the locked revision and `narHash` semantics from `flake.lock`

#### Scenario: Private/offline input can be selectively rewritten [r[vmci-nix-store-boundary.input-materialization.private-input]]

- GIVEN a flake input is explicitly classified as private, unavailable to the guest, or required for offline execution
- WHEN VMCI prepares the workspace lock file
- THEN Aspen MAY rewrite that input's locked node to a local path with a correct `narHash`
- AND Aspen MUST preserve compatible `original` identity when needed to prevent dirty lock refresh
- AND Aspen MUST NOT apply the same path rewrite to unclassified public inputs

#### Scenario: Broad path rewrite is rejected [r[vmci-nix-store-boundary.input-materialization.reject-broad-rewrite]]

- GIVEN host prefetch metadata contains entries for both a private input and a large public/cacheable input
- WHEN the VMCI lock rewrite runs
- THEN only allowlisted/private inputs are rewritten to `type = "path"`
- AND public/cacheable inputs remain on locked fetcher metadata or another explicitly VMCI-safe guest-local/cache strategy

### Requirement: VMCI Nix Store FD Pressure Diagnostics [r[vmci-nix-store-boundary.fd-pressure-diagnostics]]

Aspen MUST classify Nix failures caused by source input materialization file-descriptor pressure as a distinct VMCI boundary with bounded, redacted evidence.

#### Scenario: Too many open files during Nix source copy is classified [r[vmci-nix-store-boundary.fd-pressure-diagnostics.classify]]

- GIVEN a VMCI `ci_nix_build` job fails with `Too many open files in system` while copying, chmodding, unpacking, or reading a `/nix/store/...-source` path
- WHEN dogfood diagnostics summarize the failed pipeline
- THEN the failure MUST be classified as VMCI Nix source/store materialization FD pressure rather than route, source blob, workspace setup, timeout, or generic build failure
- AND the summary MUST include bounded path basename/hash evidence, phase marker, job id, rail/profile, and receipt/log handles
- AND the summary MUST redact secrets and avoid full unbounded paths, environments, tickets, or argv payloads

#### Scenario: Diagnostics distinguish post-command source pressure [r[vmci-nix-store-boundary.fd-pressure-diagnostics.post-command]]

- GIVEN progress markers include workspace materialization completion and `command_started`
- WHEN the Nix stderr contains the FD pressure signature
- THEN the diagnostic boundary MUST identify the failure as after executor command start inside Nix source/store handling
- AND MUST NOT recommend lower-layer VMCI route/blob/workspace fixes unless those markers are absent

### Requirement: VMCI Medium Rail Acceptance Gate [r[vmci-nix-store-boundary.medium-gate]]

The VMCI medium rail MUST be the acceptance gate before running clippy/full VMCI for this boundary.

#### Scenario: Medium passes source-input boundary [r[vmci-nix-store-boundary.medium-gate.pass]]

- GIVEN the VMCI Nix input/store strategy has been changed
- WHEN `nix run .#dogfood-local-vmci-medium` completes
- THEN the receipt MUST show VMCI startup, source push, CI trigger, workspace materialization, `format-check`, and `build-cli` final status
- AND `build-cli` MUST not fail with `Too many open files in system` while handling a large source input

#### Scenario: Medium fails at new boundary [r[vmci-nix-store-boundary.medium-gate.new-boundary]]

- GIVEN the medium rail still fails after the source-input strategy changes
- WHEN diagnostics are captured
- THEN the failure MUST identify the new boundary with bounded evidence
- AND the OpenSpec evidence MUST record why it is not the prior VMCI Nix source/store FD pressure signature
