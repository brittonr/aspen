# Live/simulation execution differential evidence — 2026-10-21

## Scope and ownership

Change: `differentiate-live-simulation-conformance`, on isolated worktree branch `cairn/next-audit-20261021`, based on `2e06fe9d0`. Immediate outcome: the differential is no longer a trace list compared with itself; one bounded shared workload is executed through each side's declared port contract, and results distinguish agreement, permitted divergence, incompleteness, and failure. Durable contribution: typed workload, outcome-class, and differential-result contracts plus separately identified structural and execution artifacts. Molten fabric-simulation maintainers own the workload admission, comparison, canonical records, and fixture.

## Pre-change observation

At the base revision, `reference_contract_differential` built `trace_refs` from the admitted port profiles and passed the same vector as `simulation_trace_refs` and `live_trace_refs`. The comparison therefore could not diverge: the structural artifact reported `equivalent = true` under all inputs. The base focused baseline was `cargo test -p molten-core fabric_simulation` 33 passed and `cargo test -p molten --lib fabric_simulation` 14 passed.

## Implementation

- `LiveWorkload` admits at most 64 bounded steps. Each step declares a port class, a canonical request ref, a non-empty sorted unique permitted outcome-class set, and an optional ordering key. Identity is BLAKE3 over a domain-separated encoding of those declarations.
- `OutcomeClass` covers delivered, rejected, backpressured, delayed, and unsupported. `WorkloadExecutor` is the single host abstraction; `execute_workload` runs every step through the selected executor.
- `evaluate_live_differential` normalizes observed outcomes, evaluates per-key ordering, and returns exactly one typed decision: agreement, permitted divergence (step pairs plus ordering), incomplete (per-step side and reason), or failure (first live class outside its declared set). Incomplete never becomes agreement.
- The shell keeps the descriptor-level structural artifact unchanged and adds `molten.fabric-simulation.execution-differential.v1`. `DifferentialEvidence` binds both artifact refs, the shared workload, both result kinds, the incomplete count, and four non-claims; it denies when both artifacts share one identity.
- The reference fixture builds its workload from the admitted manifest, maps simulated outcomes from actual routed port events, selects no live adapter, and therefore records `incomplete` for the execution side. The CLI exports both new artifacts (`execution-differential.preserves`, `differential-evidence.preserves`).

## Executed checks

- `cargo test -p molten-core fabric_simulation`: 39 passed, zero failed. Includes malformed/oversized workload denial, identity sensitivity, agreement, permitted divergence with ordering, unpermitted live failure, unsupported/not-executed/adapter-unavailable incompleteness, and executor routing.
- `cargo test -p molten --lib fabric_simulation`: 16 passed, zero failed. Includes canonical agreement/divergence/failure/incomplete identities, distinct structural/execution identities, evidence binding, and rejection of one shared identity.
- Actual CLI execution of the built binary:
  - `molten fabric-simulation run --out <owned-tmp>/run`: `world=blake3:4e30eebc… run=blake3:492053ac… choices=18 events=6 invariants=12 decision=pass`
  - `molten fabric-simulation inspect <owned-tmp>/run/report.preserves`: `profile=deterministic-whole-system decision=pass … divergence=none run=blake3:492053ac…`
  - `molten fabric-simulation export --out <owned-tmp>/export`: `differential=blake3:6876cc79… execution-differential=blake3:2b6385a5… differential-evidence=blake3:c4f51e88… artifacts=6`
  Both run and export directories contain `execution-differential.preserves` and `differential-evidence.preserves` alongside the structural differential.
- `cargo fmt --all --check` passed; `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test -p molten-core fabric_simulation`: 39 passed; `cargo test -p molten --lib fabric_simulation`: 16 passed.
- `cargo test --workspace --all-targets`: 2,281 passed across 18 suites with zero failures; `cargo test --workspace --doc`: 7 doctests passed.
- Both configured Octet producers reported `clean`: [workspace status](2026-10-21-source/workspace/status.json) and [library status](2026-10-21-source/library/status.json). Artifact import [receipt](2026-10-21-source/workspace/artifact-ledger-receipt.preserves) `blake3:0d659b4e28baf18af1820f7fd9952228cd4b8264b3e3e1fd48f9e0aabca1c57e`; unchanged `strict-ci` gate [receipt](2026-10-21-source/workspace/gate-receipt.preserves) `blake3:fa615429271d3e303ac07b9eac384dbd86a1aff862e0ec1ed5f7c1c4c0c33311`; [remediation plan](2026-10-21-source/workspace/remediation-plan.preserves) `blake3:464a73ce7a055808ca34a0d9e95492b1b993c9eefc889feb7ed30f8d658ba53e`.
- `nix flake check -L --keep-going --no-update-lock-file` exited 0 with `all checks passed!` for the exported `x86_64-linux` checks. The hermetic nextest check realized 1,807 testcases with zero failures; its [CI receipt](/nix/store/dmbwzn3scm58kz9lpvy2qgrv97jxbghp-molten-nextest/ci-test-run-receipt.preserves) records `decision="pass"`, source ref `blake3:f912c5be65cc8892ec0b41702d4640c56c4ccb3905659ebe44404a23af57404d`, and raw receipt-file BLAKE3 `27ffa5a8e2c3e2d2a4b6ad68e9cdf8c61a7efcffb6b3f3a5c7474ce4e719f625`. The raw digest is not the canonical receipt reference. The omitted incompatible systems were `aarch64-darwin`, `aarch64-linux`, and `x86_64-darwin`.

Preserved failures during this work: the first Octet pass reported seven `excessive_file_length` and two `function_length` findings, repaired by moving types/builders/fixture/tests into focused textual parts and splitting the decision encoder; a retry then reported one unused import, removed. A workspace test run under concurrent build load hit Hegel `TooSlow` health checks in three unrelated property tests; each passed alone and the full suite then passed with zero failures.

## Preserved limits

- Agreement is bounded to the exercised workload and declared properties. It does not prove live equivalence, performance, durability, or release eligibility.
- No live adapter is composed in this fixture, so the production fixture records incompleteness. Agreement and failure paths are exercised through the canonical builder with explicit recorded sides, not live networking or storage.
- The structural artifact still compares descriptors and cannot diverge by construction; it is retained as a separate identity and must not be read as execution evidence.
- Ordering divergence is evaluated per declared key over observed class sequences, not over physical timing or byte identity.

## Native lifecycle acceptance

- Strict validation passed with 34 active changes, zero issues and zero findings; receipt `8a3d179b8124fc7a6a4252e7ce9f3c1c85b86c3e14a04c4ffd121e35e404e247`.
- Proposal, design and tasks gates returned `PASS`: `35f03adcbc5825829446534298d09a43917e6addf1b7b3223be6108b72634155`, `ec142978f32eebcc56b7fc9e89eaad2216f687500a2f0920044eeee5ec8c60e2`, `358b06e52b4405428af61b22c6536382c4fe5867013d78e28111dd74bfff9434`. The [completeness review](2026-10-21-review-completeness.json) returned `PASS`, receipt `db8cff82fc9c2315ff508c626e9d321a58774a43b795c9f37e18d217ae9fdbdd`.
- [Sync](2026-10-21-sync-receipt.json) merged the delta spec into `.cairn/specs/fabric-simulation/spec.md` with `blocked=false`, receipt `2d79b8abb5cd1160a244a05510c27cefbcb90abfac1024cf7c74dc9f0f9844ba`.
- [Archive](2026-10-21-archive-receipt.json) reported `blocked=false`, receipt `e2eff3888713000ec7e789ded92a8cd6ccf40c3ab35aba59674b0c020a2d9bad`; the change now lives at `.cairn/archive/1970-01-01-differentiate-live-simulation-conformance/`.
- Post-archive strict validation passed with 33 active changes, zero issues and zero findings, receipt `ff311fc632009d5d76a101c8cb1296b3566670c5c8952616047da0ac287d5b5a`.

These structural results do not replace the executed behavior evidence above.
