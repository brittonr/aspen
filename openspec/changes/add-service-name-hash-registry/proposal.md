## Why

Unison separates immutable service/code hashes from mutable human names. Aspen has Forge, CI, deploy, dogfood, and runtime-service surfaces that need the same distinction: operators need stable names, while receipts and rollback need immutable deployed identity.

Aspen should add a Raft-backed registry where service names point to immutable service hashes / closure hashes, so deploys, rollbacks, and receipts can cite both the mutable pointer and the exact content-addressed target.

## What Changes

- Define `ServiceHash` as immutable deployed-service identity, initially backed by execution closure or artifact manifest hash.
- Define `ServiceName` as a Raft-controlled mutable pointer to a `ServiceHash` with generation and update receipt.
- Require lookup/readback APIs to distinguish name, resolved hash, generation, and previous hash.
- Require deploy/update/rollback receipts that do not expose credentials.

## In Scope

- Runtime-service and deploy-facing registry contract.
- Name assignment, update, rollback, lookup, and receipt behavior.
- Conflict and authorization requirements.

## Out of Scope

- Public HTTP service routing.
- Replacing Forge Git refs.
- Full application marketplace/install lifecycle.

## Verification

- `openspec validate add-service-name-hash-registry --strict`
- Focused registry unit tests for assign, update, rollback, conflict, and authorization failures.
- Runtime-service/deploy receipt tests.
- `openspec validate --all --strict --json`
- `git diff --check`
