## ADDED Requirements

### Requirement: Conservative local VM check scheduling [r[test-suite-metadata.vm-check-scheduling]]

Aspen's repository flake configuration MUST provide a deterministic local default for `nix flake check` that does not silently over-schedule heavyweight NixOS VM tests on a single host.

#### Scenario: Full flake check defaults to serialized local jobs [r[test-suite-metadata.vm-check-scheduling.serial-default]]

- GIVEN an operator runs `nix flake check -L` in the Aspen checkout and accepts repository flake configuration
- WHEN Nix reads the flake `nixConfig`
- THEN the default `max-jobs` value SHALL serialize local jobs unless the operator explicitly overrides it
- AND the flake source SHALL document that the setting protects VM-test reliability rather than product runtime semantics

#### Scenario: Focused VM checks remain available [r[test-suite-metadata.vm-check-scheduling.focused-checks]]

- GIVEN a full flake run reports a VM-test timeout or guest-startup failure under operator-overridden parallelism
- WHEN the failure is triaged
- THEN the affected focused check SHALL remain runnable as a direct `nix build .#checks.<system>.<name> --no-link -L` target
- AND a focused pass SHALL be valid evidence that the prior full-run failure was scheduling/resource contention rather than a deterministic product failure

### Requirement: VM evidence classification policy [r[dogfood-evidence.vm-scheduling-classification]]

Aspen acceptance evidence MUST distinguish VM scheduling/resource contention from deterministic product failures.

#### Scenario: Parallel contention is not promoted as product failure [r[dogfood-evidence.vm-scheduling-classification.parallel-contention]]

- GIVEN a parallel full-flake run fails in a VM check with host contention symptoms such as guest shell startup timeout, VM boot starvation, or short Raft RPC timeouts during overloaded test startup
- WHEN the corresponding focused VM check passes and a serialized full rail passes
- THEN the evidence record SHALL classify the parallel failure as local scheduling contention
- AND the remediation SHALL prefer scheduling/resource guardrails over product code changes
