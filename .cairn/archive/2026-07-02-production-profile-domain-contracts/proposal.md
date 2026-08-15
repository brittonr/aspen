# Change: production-profile-domain-contracts

## Why

The checked-in production node profile currently treats evidence refs, profile names, paths, and resource limits as plain `String` or `Number` values. That catches only gross shape errors and lets typoed refs, unsafe paths, empty identifiers, fractional limits, and zero-capacity limits pass Nickel export before later evidence gates discover the problem.

## What

- Replace loose `Ref`, `Text`, and `Number` profile aliases with domain-specific Nickel contracts for BLAKE3 content refs, non-empty text, absolute state roots, safe relative layout directories, and positive integer limits.
- Make invalid scalar values fail during `nickel export docs/production-node-profile.ncl` with focused diagnostics.
- Preserve the existing exported profile shape for valid values so downstream receipt generation continues to consume the same fields.

## Impact

Production profile review becomes fail-closed earlier. Operators see Nickel contract failures before binding malformed profile inputs into production readiness receipts.
