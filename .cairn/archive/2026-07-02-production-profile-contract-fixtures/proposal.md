# Change: production-profile-contract-fixtures

## Why

Tightening Nickel contracts is risky without executable examples that prove both accepted and rejected profile shapes. The current production runbook tells operators to export the profile, but there is no dedicated positive/negative fixture suite for profile contract behavior.

## What

- Add production profile Nickel fixtures for the valid checked-in profile and representative invalid profiles.
- Run fixture validation in the smallest relevant check so contract regressions fail before production readiness receipts change.
- Cover malformed refs, empty arrays, duplicate or missing adapters, unsafe paths, resource invariant failures, unsupported metadata, and vocabulary typos.

## Impact

Contract changes become reviewable through deterministic pass/fail fixtures. Future profile hardening can prove it rejects bad inputs without breaking the reviewed valid profile export.
