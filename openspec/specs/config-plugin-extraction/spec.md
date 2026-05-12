# config-plugin-extraction Specification

## Purpose

Define the minimum manifest, ownership, feature-contract, and evidence rails required before config/plugin extraction candidates move beyond workspace-internal readiness.

## Requirements

### Requirement: Config/plugin extraction manifest exists before readiness work

The config/plugin family MUST have an owner, manifest, feature contract, representative consumers, dependency exceptions, and verification rails before any readiness state is raised beyond `workspace-internal`.

ID: config-plugin-extraction.manifest-first

#### Scenario: Inventory has no ownerless manifest gap

- GIVEN the broader crate extraction inventory
- WHEN the config/plugin family is listed
- THEN it MUST link to a manifest and identify the owning maintainer group even if readiness remains `workspace-internal`.

#### Scenario: Standalone examples define the reusable contract

- GIVEN `aspen-nickel` and `aspen-plugin-api` as candidate reusable crates
- WHEN standalone examples and feature minima are documented
- THEN the manifest MUST distinguish reusable config/protocol APIs from runtime plugin host integration.
