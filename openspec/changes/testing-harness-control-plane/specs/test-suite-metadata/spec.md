## ADDED Requirements

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
