## Why

Dogfood receipts now give operators durable evidence for the full self-hosting loop, and runtime-host product-path tests emit row-specific evidence markers. Those receipt shapes are still scattered: dogfood run receipts, CI run receipts, job outputs, and runtime-host proof logs each answer part of the same operator question: what ran, under which artifact and host identity, with what result, and where is the bounded evidence?

Aspen should generalize runtime application receipts so service/runtime/job evidence can be queried without log scraping or chat-only context.

## What Changes

- Define a common runtime application receipt contract for Aspen-started units.
- Require host/artifact/run identity, lifecycle status, bounded output handles, and redacted diagnostics.
- Require generated typed validation for canonical receipt JSON when the owning Rust type exists.
- Connect dogfood/CI/job/runtime-host evidence without replacing the existing dogfood receipt schema.

## In Scope

- OpenSpec requirements for a generalized receipt model and readback behavior.
- Secret-redaction, bounded-output, deterministic-ordering, and validation expectations.
- Migration guidance that preserves existing dogfood and CI receipt compatibility.

## Out of Scope

- Replacing the current dogfood receipt schema in this spec-only slice.
- Building a new UI or persistent query service immediately.
- Serializing raw logs, secrets, capability tokens, or cluster cookies into receipts.

## Verification

- `openspec validate generalize-runtime-application-receipts --strict`
- `openspec validate --all --strict --json`
- `git diff --check`
