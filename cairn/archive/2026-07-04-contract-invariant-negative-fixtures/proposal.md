# Change: contract-invariant-negative-fixtures

## Why

Production profile contracts have a useful positive/negative fixture matrix, but other contract surfaces have thinner negative coverage. When each invariant lacks its own failing fixture, regressions can accidentally accept malformed refs, missing evidence, duplicate ids, stale metadata, invalid enums, or cross-field contradictions.

## What

- Establish a reusable fixture coverage rule for Nickel contract modules: every exported invariant needs at least one positive path and one focused negative path, or an explicit exemption.
- Add missing negative fixtures for plugin extension contracts, grants, peer profiles, multinode scenarios, production profiles, and Cairn policy cross-reference invariants.
- Prefer one violated invariant per negative fixture so failure classes remain easy to diagnose.

## Impact

Contract tests better demonstrate fail-closed behavior. Reviewers can see which malformed inputs are expected to fail before exports or receipts are refreshed.
