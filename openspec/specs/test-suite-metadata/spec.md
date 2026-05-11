# test-suite-metadata Specification

## Purpose

Defines the Test Suite Metadata capability requirements preserved by Aspen's archived OpenSpec records, including nickel suite manifests are authoritative, generated inventory drives suite grouping and registration.
## Requirements
### Requirement: Nickel suite manifests are authoritative

The test harness SHALL define suite metadata in Nickel. Each suite manifest SHALL declare a stable suite identifier, execution layer, owner, runtime class, prerequisites, tags, and execution target. Generated inventory outputs consumed by Rust and Nix tooling SHALL be derived from those Nickel manifests rather than maintained by hand in multiple places.

#### Scenario: Suite metadata exports a complete inventory record

- **WHEN** a maintainer adds or updates a suite manifest
- **THEN** metadata validation SHALL export an inventory record containing the suite identifier, layer, owner, runtime class, prerequisites, tags, and execution target

#### Scenario: Invalid metadata blocks stale harness output

- **WHEN** a suite manifest is missing required fields or contains an unsupported layer or prerequisite
- **THEN** metadata generation SHALL fail before nextest, flake checks, or reporting consume outdated generated outputs

### Requirement: Generated inventory drives suite grouping and registration

The harness SHALL generate machine-consumable outputs from the Nickel manifests for Rust and Nix entry points. Grouping, filtering, and check registration SHALL resolve from that generated inventory instead of duplicating suite lists and grouping rules across shell scripts, flake check declarations, and hand-written filters.

#### Scenario: VM suite registration is generated from metadata

- **WHEN** a suite manifest declares a `vm` layer suite with flake execution attributes
- **THEN** flake check registration SHALL be derived from generated inventory data instead of a hand-maintained per-suite registration block

#### Scenario: Focused suite selection resolves from generated inventory

- **WHEN** a developer selects suites by layer, tag, owner, or runtime class
- **THEN** the harness SHALL resolve the selection from generated inventory data without requiring duplicate grouping rules in multiple configuration files

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
