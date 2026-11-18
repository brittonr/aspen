# Shared construction route, 2026-11-18

Change: consume Choregraph's published `choregraph.graph-construction.v1`
library (Choregraph `c3577352ee1e39391f5204ee234d991c7dbeb151`) through the
nickel-eval captured-module evaluator (nickel-eval
`2d1fb2b59f373967cc69e13ec289759d9cd4df46`) behind the opt-in
`job-dag-construction` feature. Base: `origin/molten` `388e567f7`.

## Executed checks

All commands ran in `nix develop --no-update-lock-file` on the final tree.

| Check | Result |
| --- | --- |
| `cargo fmt --check` | passed |
| `cargo clippy --all-targets -- -D warnings` | passed |
| `cargo clippy --all-targets --features job-dag-construction -- -D warnings` | passed |
| `cargo test --features job-dag-construction --test job_dag_construction` | `test result: ok. 17 passed; 0 failed` (8 construction tests, 9 shared test-support tests) |
| `cargo test --lib capability_qualification` | `test result: ok. 7 passed; 0 failed` |
| `cargo octet check --artifact-dir …/octet` | `Status: clean`, `Findings: 0`; profile `b3:c95fda09a7434ceb1ff9ea0ba621213e6cf95f41a099b3ba7b5c28b4f90cc2d5` |
| `cargo octet check -p molten --artifact-dir …/octet-lib -- --lib` | `Status: clean`, `Findings: 0`; profile `b3:e1ece9b00492378261628c312dc74cc82ec73d9e9e4974903cd349e525a84fd5` |
| `cargo octet check -p molten --artifact-dir …/octet-feature -- --all-targets --features job-dag-construction` | `Status: clean`, `Findings: 0`; profile `b3:f4dd0895670c5d7c4ea37a457c9cabab9a66bc910d0afe91e7a923a8833ea5a9` |
| `nix build .#checks.x86_64-linux.nextest` (`/nix/store/i9gllzfllww75501hc1nvgn84pwb15gv-molten-nextest.drv`), local builder | `2480 tests run: 2480 passed, 37 skipped` |
| strict Cairn validation with the sibling Cairn policy | valid, 0 issues, 0 findings, 11 active changes |

The construction tests cover sequence, shared-diamond, reusable-template,
zero-data dependency and independent canonical-fixture lowering. Negative
controls cover unknown operations, a foreign registry, fractional values,
fan-in usage, cycles, duplicate keys, effects, ambient imports, a missing
parameter, a dangling output, a reserved module identity and an unselected
worker identity. The node budget admits 64 nodes and rejects 272 with
Choregraph's `LimitExceeded`.

## Repairs found during verification

- The first feature Octet run reported 39 findings: file and function length,
  non-trait imports, unbounded map growth, boolean names and an implicit
  external default. The route now spans `construction.rs`,
  `construction/registry.rs` and `construction/lower.rs`; the lowerer identity
  hashes all three. Unnarrowed Choregraph ceilings use its reviewed default
  values explicitly, so admission bounds are unchanged.
- The first hermetic run failed six `capability_qualification` tests with
  `compiled feature inventory drift`: the new Cargo feature needed an explicit
  qualification cohort entry.

## Environment observation

On the remote `aspen1` builder, the same derivation ran 2,479 tests and then
timed out `blocks::semantic_store::tests::hegel_chunk_store_determinism_range_resumable_and_no_dangling`
at the CI profile's 180 s limit, twice. That test, which writes chunked stores
for 32 cases, was already the suite's slow test in October (78 s). It took
8.1 s in a local debug run, and the local build of the same derivation passed.
This change does not touch the chunk store. The timeout is recorded as a
builder observation, not as a pass on that builder.

## Non-claims

Lowering never executes a stage; only the tests' direct-graph comparisons run
the selected-sequence runtime. Choregraph elaboration does not make Aspen stage
semantics equal to Choregraph machine semantics. A worker identity is integrity
of the selected bytes, not executable authorization. Ordered `concat-u64`
fan-in, effects and cross-target laws stay outside this route.
