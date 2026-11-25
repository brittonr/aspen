# Catalog deltas

## ADDED Requirements

### Requirement: Catalog queries, upgrade plans, plugin admission, secret policies, and peer bootstrap live in a no_std kernel
r[molten.catalog.no_std_catalog_upgrades_and_policy_kernel] Pure catalog query admission, upgrade quarantine/migration planning, plugin manifest admission, secret-policy classification/redaction, and peer bootstrap negotiation MUST live in a `#![no_std]` + `alloc` crate (`molten-catalog-upgrades-core`) atop `molten-preserves-core` without `std::path::PathBuf` or live I/O.

#### Scenario: Plugin admission and upgrade planning run under no_std
- GIVEN a plugin manifest and upgrade migration request in canonical packed form
- WHEN `molten-catalog-upgrades-core` evaluates them under `#![no_std]`
- THEN it MUST produce deterministic allow/deny plans and quarantine transitions without OS I/O.
