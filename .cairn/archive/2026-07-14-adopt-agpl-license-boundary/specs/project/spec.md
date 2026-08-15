# Project License Boundary Specification

## Purpose

Align Molten's network-runtime distribution metadata with its AGPL component closure while preserving third-party terms.

## Requirements

### Requirement: Molten-owned packages declare AGPL

r[molten.project.license_boundary.package_metadata] Molten-owned Rust packages MUST declare `AGPL-3.0-or-later` in repository-owned package metadata.

#### Scenario: Package metadata is inspected

- GIVEN a distributor inspects Molten and `molten-core` package metadata
- WHEN the selected project license is read
- THEN each Molten-owned package MUST report `AGPL-3.0-or-later`.

### Requirement: License artifacts accompany source

r[molten.project.license_boundary.license_artifacts] Molten MUST ship the complete AGPL-3.0-or-later license text and MUST identify that third-party and vendored material remains governed by its original terms.

#### Scenario: Source archive is distributed

- GIVEN a source archive contains Molten-owned and vendored material
- WHEN a recipient reviews its license artifacts
- THEN the archive MUST include the AGPL text and MUST NOT represent vendored code as relicensed Molten-owned code.

### Requirement: Current documentation states the selected boundary

r[molten.project.license_boundary.documentation] Current Molten documentation MUST state the AGPL boundary and MUST NOT claim that the license selection revokes earlier grants or proves legal compliance in every jurisdiction.

#### Scenario: A reviewer reads the architecture boundary

- GIVEN current architecture documentation describes Molten and historical Aspen material
- WHEN licensing is discussed
- THEN it MUST distinguish project-owned AGPL source from separately licensed third-party material without retaining a contradictory permissive project declaration.

### Requirement: Generated package metadata remains fresh

r[molten.project.license_boundary.generated_metadata] Checked-in generated package metadata MUST agree with the repository-owned Cargo manifests for Molten package license expressions.

#### Scenario: A stale build plan retains the permissive expression

- GIVEN Cargo metadata declares AGPL while a generated Molten package row declares MIT or Apache
- WHEN freshness validation runs
- THEN validation MUST fail until the generated row is refreshed.

### Requirement: License boundary validation is deterministic

r[molten.project.license_boundary.final_validation] The repository MUST validate the selected package expressions, required license artifacts, current documentation, and absence of contradictory project-owned license declarations.

#### Scenario: A project-owned declaration drifts

- GIVEN one current project-owned metadata or documentation surface declares a conflicting project license
- WHEN the focused license audit runs
- THEN the audit MUST fail without treating dependency license expressions as project drift.
