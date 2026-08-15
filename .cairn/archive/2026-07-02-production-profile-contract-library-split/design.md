# Design: Production profile contract library split

## Context

A reusable Nickel contract library avoids duplicating the production profile schema across docs, fixtures, tests, and future node profiles. It also makes it clear which file owns schema policy and which file owns a concrete deployment profile.

## Layout

Create a contract module for the production node profile schema and scalar helpers. Keep `docs/production-node-profile.ncl` as an instance file that imports the module, applies `ProductionNodeProfile`, and contains the reviewed pilot values.

A typical layout is:

- `contracts/production-node-profile.ncl` for reusable contracts and constants.
- `docs/production-node-profile.ncl` for the concrete pilot profile instance.
- fixture files for positive and negative examples that import the same contract module.

## Export boundary

The runtime continues to consume exported JSON. Nickel remains a development and review-time validation step. No runtime Nickel interpreter is introduced for node startup.

## Documentation

The runbook should tell operators which file to edit for a concrete profile, which file owns the reusable contract, and which export command produces the reviewed JSON artifact.
