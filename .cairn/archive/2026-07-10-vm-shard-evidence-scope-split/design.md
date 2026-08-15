# Design: vm-shard-evidence-scope-split

## Overview

Introduce an explicit evidence-scope split for VM shard artifacts. Synthetic shard metadata and executable VM observations may both be reviewable, but they must not satisfy the same claims unless a gate explicitly permits the scope.

## Functional core and shell boundary

The pure core classifies shard evidence from explicit fields: shard id, artifact kind, evidence scope, child refs, diagnostic-log refs, executable VM receipt refs, host-support state, and caveats. Aggregation validates scopes without reading files.

The shell owns Nix builders, VM execution, generated fixture artifacts, and file copying.

## Scope classes

Initial scope classes:

- fixture-metadata: validates command/profile wiring and expected artifact declarations;
- executable-vm: binds NixOS VM child receipts from actual VM execution;
- aggregate-index: indexes child refs without upgrading their scope;
- diagnostic-only: logs or unavailable evidence that cannot pass platform claims.

## Aggregate behavior

Aggregates must preserve each child scope and deny when a required executable platform claim is backed only by synthetic refs, logs, or unavailable evidence.

## Boundaries

This change clarifies evidence classification. It does not change subsystem authority, policy, provenance, or source-gate requirements.
