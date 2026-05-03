## Why

Dogfood receipts now preserve operator evidence, but operators still need to manually translate failed stage/category data into first triage steps. The documented runbook should be executable so failed self-hosting evidence points at the next bounded investigation without relying on memory.

## What Changes

- Add `aspen-dogfood receipts diagnose <run-id-or-path>`.
- Reuse existing validated receipt loading and selector behavior.
- Print a deterministic operator diagnosis: overall status, failed stage, failure category/message, and recommended first checks.
- Keep the command read-only and receipt-local; it must not require a running cluster or mutate receipt files.

## Impact

- **Files**: `crates/aspen-dogfood/src/main.rs`, docs, and dogfood evidence spec.
- **APIs**: CLI subcommand addition under `receipts`.
- **Testing**: dogfood package tests plus a smoke run against the latest successful receipt.
