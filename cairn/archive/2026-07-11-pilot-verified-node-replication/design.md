# Design: Pilot verified node replication

## Pinned inputs

The pilot records the verified-node-replication revision and Nix source hash, plus the exact Verus submodule revision and source hash. The source is admitted only when its MIT license, crate manifest, proof root, and counter example sentinels are present.

## Compatibility probe

The shell invokes Octet's pinned production Verus with explicit argv and `-V new-mut-ref`. The probe captures bounded stdout/stderr and classifies the result:

- `compatible`: verification succeeds and the expected verification summary is present;
- `blocked-verifier`: verifier exits unsuccessfully or reports an internal error;
- `blocked-profile`: source/profile identity or claim-boundary validation fails.

The current observed route is `blocked-verifier`: without `new-mut-ref` the current verifier rejects the source, and with it the verifier reaches an internal `var_local_id` failure in `exec/rwlock.rs`. This is a valid pilot result, not a reason to synthesize runtime adoption.

## Trusted-boundary audit

The pilot counts and records `#[verus::trusted]` markers in the pinned source, including top-level refinement theorems and trusted traits. A passing upstream verifier result would still not erase these assumptions. Promotion requires a reviewed trusted-boundary account and a locally scoped proof receipt.

## Promotion gate

Runtime dependency work may begin only after:

- exact verifier compatibility succeeds without internal errors;
- trusted boundaries are cataloged and accepted;
- a bounded API adapter design keeps NUMA-local semantics distinct from distributed fabric semantics;
- positive and negative concurrency tests exist;
- a NUMA benchmark profile and rollback path are declared;
- Octet, provenance, and Cairn evidence remain scoped.

## Functional boundary

Typed profile validation and decision classification are pure. Nix owns source fetching and verifier process execution. No external source code enters Molten runtime crates during this pilot.
