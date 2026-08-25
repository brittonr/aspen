# Molten 0.1.0 limited internal pilot

<!-- r[verify molten.prod_release.pilot_non_claims] -->

Release date: 2026-08-25

Molten `0.1.0` is a limited internal pilot. It is not a general production release.

## Candidate

- Commit: `a4f111690b6962f04d9320fd93d09c7dd1ad2fd0`
- Git tree: `58a6763c3668121ffa7309195f8d2c76ef4950d3`
- Source reference: `blake3:80e3ceb18784504c7573191fce72e121d0789613c6c5f7bdcecbdd9ae0e4cdb7`
- Tag: `v0.1.0`

The source reference covers the framed commit and tree identity. It does not cover external dependencies or release authority.

## Pilot workloads

The pilot permits these workloads:

- stateless internal jobs;
- content-addressed local jobs;
- reviewed two-node VM workflows;
- local dogfood and evidence export.

The pilot excludes these workloads:

- customer-critical workloads;
- destructive retention operations;
- production consensus;
- real-WAN deployment.

## Current capabilities

Molten can run deterministic actor turns, dataspace assertions, supervised services, reviewed Wasm, reviewed Steel, local jobs, and Iroh workflows.

It can enforce explicit capability, policy, provenance, source, and resource gates before protected effects.

It can produce canonical Preserves receipts, signed evidence bundles, release promotion records, and verified portable exports.

## Validation

- Rust and nextest: 1,418 passed, 0 failed, 0 skipped.
- CI receipt: `blake3:d59616502a20cee1d48447db4fbdf1d8fc2edcbc09b78838b408d50efa139044`.
- Strict Cairn validation passed.
- Dogfood report: `blake3:83cabd032ebee3f080c86bef40f4077b5556d0d78be76d70a92da4c841b4e407`.
- Dogfood verification: `blake3:bb8a69e0647451bfb54c49082a4ddf905859ddad2c9cc45dc7c328cc941f9581`.
- Signed bundle verification: `blake3:833d35e693d20fc5ea34c60ba940e233eeb6a3eb3fb8770262c077f5b9a9d0a6`.
- Promotion: `blake3:74a49eb3674ac39c21d798b682a70aa48ba0cd972c87c39e435768c5c4346125`.
- Export verification: `blake3:5176eac8757539a7e75015d62844c79cff3c5c362ef3c925ccb405ef0db6dc0c`.
- VM validation: `blake3:f41c58d226085d33b0accbc8395f5ea0776be6ca97d76c3cea6e13c4ac65c78a`.
- Pilot decision: `blake3:ea03ddbada910b33a09fe3156e9908b2b7c1f1b0c6b14b39570ae05abdaf27cb`.
- Candidate gate: `blake3:9d15e7f99a34ba5f1a979d07cd7023025347144349db50caacca6ddbb90ab1e4`.

The reviewed artifacts are preserved with the archived Cairn change. The typed manifest is `release/molten-0.1.0-pilot.ncl`.

## Required caveats

Pinned Octet has current configuration identity, but its strict gate denies 5,771 warning findings.

The pilot binds that deny receipt. It does not treat warning-only output as strict source-gate acceptance.

The VM network-control fault is unavailable. Its receipt does not claim successful network fault injection.

The release profile uses the pilot tier. No reviewed Valence release-policy hash is selected for release-tier validation.

## Rollback triggers

Rollback the pilot if any condition occurs:

- source-gate evidence becomes stale;
- dogfood replay fails;
- VM evidence drifts;
- an authority denial regresses.

## Stop conditions

Stop the pilot if any condition occurs:

- the candidate gate fails;
- a node cannot recover after restart;
- a canonical receipt or signature fails verification.

## Non-claims

This pilot does not establish production consensus, real-WAN behavior, sustained SLOs, fleet-scale pressure, adversarial security, or destructive-operation readiness.
