# Require a production profile candidate input

## Why

The checked-in production node profile contains an all-zero source-gate reference. That placeholder can be exported without naming the source candidate under review.

## What Changes

- Make the production profile require an explicit candidate source reference.
- Reject all-zero and repeated dummy candidate references in the Nickel contract.
- Keep deterministic positive fixtures with an explicit fixture reference.
- Make Nix checks prove missing and placeholder candidate denial.
- Update operator commands to supply the reviewed candidate reference.

## Impact

Operators must provide `candidate_source_ref` when they export `docs/production-node-profile.ncl`. This change does not generate candidate evidence or establish release eligibility.
