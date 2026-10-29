# Shared component adoption: frozen mapping and execution — 2026-10-26

## Disposition

Cutover is blocked, not complete. The immutable prerequisites are integrated,
but the exercised candidate changes callback scheduling and memory admission.
The complete local runtime remains the only production execution path. The
shared adapter is mounted only under `cfg(test)`; it is not a fallback or an
installed actor/extension backend. Do not sync or archive this change.

Immediate outcome: reproducible local/shared observations and an explicit
cutover stop. Durable contribution: locked producer contracts, a guarded
published-materialization intake and a replay-sensitive parity corpus.
Molten component-runtime maintainers own the mapping, product admission,
materialization and consumer acceptance; hardened-runtime maintainers own
execution mechanics; Mantle maintainers own their producer schema/verifier.

## Immutable prerequisites

- `hardened-wasmtime-runtime` and `hardened-wasmtime-shell`:
  `6381322a68407da71325c4c33113d11ada9af67f`, `hwr.shell.v3`, Wasmtime 45.0.0.
  Cargo and Nix use the delegate-qualified canonical Radicle source already
  recorded in the [resumable prerequisite handoff](2026-10-26-resumable-prerequisite-handoff.md).
- `mantle-wasm-consumer-verifier` and its implementation core:
  `c99556e5c91dfd13fcc261dc451f471359d47c38`, from the canonical Mantle Git source.
  This published descendant contains the completed consumer-verifier archive
  and compatible BLAKE3 1.8.5; the earlier archive commit selected BLAKE3 1.8.2.
- `Cargo.toml`, `Cargo.lock`, both Nix inputs/lock, the Nickel release dependency
  profile, source hashes, both regenerated unit2nix plans and license notices
  agree on those cohorts. No sibling/path dependency or source override is a
  product input. Both completed native archives and exact evidence BLAKE3s are
  required by the release dependency gate.
- Native `.cairn/` archive paths are admitted by the Nickel contract; traversal
  and delegate traversal have negative fixtures. The pure Rust path validator
  already admitted safe native relative paths and required no change.

The executed Nix `release-dependency-profile` check built the actual policy
binary, validated all four pinned archive sources and passed the positive and
negative Nickel fixture loop. `scripts/git-source-hashes.sh --check` passed;
unit2nix regenerated the workspace and release-policy plans from the lock.
These are dependency/linkage facts, not upstream correctness or adoption proof.

## Frozen request and outcome mapping

The candidate is `src/wasm/component/runtime/shared/`; executable cases are
`src/wasm/component/tests/parity/` and the existing component fixture modules.
The synchronous and time-sliced product profiles, their generated exports and
`wit/molten-component-runtime/runtime.wit` are unchanged.

| Boundary | Exact mapping / retained owner |
|---|---|
| Request admission | Existing `shell::admit_request` retains profile, migration, materialization, declared imports, grants, artifact reinspection and canonical input decisions. No shared success grants permission. |
| Feature configuration | Every product feature and determinism field is explicitly mapped into the published feature policy; no clipped values or incidental defaults. |
| Artifact | Exact portable bytes and BLAKE3 identity; admitted WIT package/world and source identity become declared interface facts. Molten independently checks actual world/import facts. No core-module or AOT fallback. |
| Resources | Checked integer/page conversions bind the supplied product limits. Current one-bound translation does not preserve the per-memory/aggregate distinction; see the executed counterexample. |
| Imports | Initial cohort has none, descriptors are empty, unexpected host callbacks deny, and no WASI linker is installed. The new product effect-request world is not implemented by this candidate. |
| Instantiation failure | Typed `InvocationPhase::Instantiation` becomes `component-instantiation-denied`, with Inspection → Denial. No successful Instantiation receipt is fabricated. |
| Invocation | Typed Invocation phase proves successful instantiation; receipts become Inspection → Instantiation → Execution or Denial. |
| Invocation failure | Guest denial maps to `guest-denial`; trap/pointer/memory/runtime/host/deadline failures map to `component-trap`, except the existing zero-remaining-fuel precedence for invocation errors. Fuel exhaustion maps to `fuel-exhausted`; input/output/concurrency limits map to `resource-denial`. Raw diagnostic strings never classify canonical receipts. |
| Canonical output | Successful shared bytes still undergo Molten's bounded canonical Preserves decoding and output BLAKE3 calculation. Invalid Preserves denies. |
| Receipts | Existing product receipt planners create the complete typed inputs, canonical payload, parent chain and identity. Successful final fuel is measured after cleanup. Denial receipts retain `fuel_remaining=None`; failed-run fuel is not erased from a successful record or invented for a denial. |
| Continuation | One retained upstream store/future; existing continuation identity and requeue command, configured quantum observations and terminal measured fuel. Dropping the invocation releases its permit. Instantiation yields are not distinguishable through the current public yielded observation. |
| Deadline | Candidate requests a positive exact-engine epoch budget. The exercised corpus did not advance that service and does not prove operated-deadline parity. Existing local component execution exposes no epoch-service API; product deadline/lifecycle composition remains open. |
| Authority/consumers | Policy, Basalt/UCAN, generation, resource and replay admission, closed effect operations, actors and extensions remain Molten-owned. Enum-labelled fixtures are not non-test product consumer evidence. |

Whole `ExecutionRecord` equality and `replay_receipts_match` compare receipt
inputs, canonical payloads, identities, parents and successful fuel, not only
stdout, decision classes or output bytes.

## Observed incompatibilities

### Start-function work changes scheduling, not final receipt fuel

An admitted import-free fixed-memory component runs a 3000-iteration decrement
loop in its core start function, then successfully echoes canonical input.
Quantum is 1000 and total fuel is 100000.

Observed local exposed boundaries: one yield, index 0, configured fuel 1000.
Observed shared exposed boundaries: fifteen yields, indices 0–14, each 1000.
The local final ledger has boundaries `[1000, 14016]`, next index 2; the shared
ledger has fifteen 1000 entries followed by 16, next index 16. Both measure
15016 consumed fuel. Complete terminal execution receipts and output agree;
the callback scheduler/replay ledgers do not.

The local shell drains instantiation internally before exposing callback
slices. The shared retained future exposes instantiation polls as callback
polls. Terminal typed phase cannot classify earlier yielded polls. A compatible
published boundary must expose a bounded instantiation checkpoint or sufficient
typed poll-phase/accounting information to preserve the existing callback
boundary. Do not classify by guest hash, instruction cost or diagnostic text,
restart the invocation, substitute a wall clock, or keep a second local engine
inside the shared adapter. A prospective corrected API still needs a new
immutable publication and these exact dual-run cases.

### Per-memory admission differs from the shared aggregate cap

The admitted component has two fixed 129-page memories in two core instances.
Each is below the product's 256-page per-memory cap; their total is 258 pages.
The independently inspected facts describe the maximum per-memory size and the
two memories/instances.

Local execution passes with Inspection → Instantiation → Execution. The
current shared mapping denies during instantiation with Inspection → Denial
and `component-instantiation-denied`. Component identity is
`blake3:4d52dbd75ca0e6f47a8c3d82dd8967525a97ed43712209051dc20465f042a95f`.
The local final receipt is
`blake3:e080a8f638bd5044e83c2c2e92950803a714d69a64b09b664c640402bc871ba0`;
the shared denial receipt is
`blake3:7b3fa94f7276bbd633480121c2599c968f65d2f0a24ebf7e1d2af1c4de730b29`.

A reviewed mapping must preserve both per-memory and aggregate meanings,
including negative cases, before cutover. No profile bound was multiplied or
widened, and no previously admitted component was removed to force agreement.

### The predicted transfer-precedence mismatch was not observed

The initial test predicted `resource-denial` for a one-page memory returning
pointer 0 and length 65537. Execution instead produced `component-trap` on both
paths; the published trap classifier checks typed Wasmtime traps before
transfer-exhaustion errors. The corrected regression compares complete records.
The paired valid span in two-page memory produces `resource-denial` on both.
No product outcome was changed to satisfy an incorrect expected mismatch.

## Exercised materialization boundary

`VerifiedBundle::load` retains the actual producer DTO and bundle identity. It
requires the published Structural report, exact admitted profile and WIT, an
independently remeasured private snapshot of every declared member, and the
actual published Structural+Bytes report agreeing with those measurements.
It does not fabricate a byte layer or translate the producer into the legacy
Molten DTO. Source opens are capability-relative/no-follow; reused stages deny;
ordinary success/error removes only the owned private child. Callers must
exclusively lease the staging namespace. Linux `/proc/self/fd` bridges the live
private descriptor to the published path-based verifier.

Focused cases exercised all-role measurements and immutable executable
snapshots, duplicate-path identity/length conflicts, non-executable member
loss/drift, parent/leaf symlinks, traversal, profile byte limits, structural
rejection, actual producer structural success followed by missing-byte Blocked,
and stage reuse/owned cleanup. This is not a successful matching production
Mantle materialization or actor/extension installation.

The real producer fixture comes from the pinned verifier's test corpus; its
source BLAKE3 is
`b06973e9d8c596225635851c5edc376ee6f6ddefbe9054eedca4be94eeca90d2`.
The producer does not ship its complete member tree and its runtime profile is
not the admitted Molten profile. It is a genuine negative source, not a
synthetic positive production bundle.

A temporary executable called the public intake directly. Observed errors were
`ComponentAdmissionDenial` for malformed JSON, the actual fixture's mismatched
Molten profile, and an occupied staging lease. Assertions confirmed removal of
owned failed children and preservation of `occupied/keep`. Exact final output:

```text
cleanup=owned-only producer-fixture=unmodified matching-production-bundle=false adoption=false
```

The executable and its RAII-owned scratch were removed after the run. Logs are
observations, not policy authority or canonical production receipts.

## Completion limits

The first post-integration focused selection passed 55 cases. It included the
ten executable candidate parity/counterexample tests and seven published
materialization cases; it did not establish complete actor/extension,
hostcall, deadline, production bundle or cutover acceptance. Subsequent
source-policy refactoring and additional negative parity cases require the
final checks recorded below, not reuse of an earlier source snapshot.

Only prerequisite integration and the frozen mapping can close at this stop.
Adapter activation, full positive/negative parity, the new closed effect world,
admitted/denied effect fixtures, real actor and extension composition, actual
matching Mantle bundles, coupled cutover/rollback, removal of duplicate mechanism
and end-to-end acceptance remain open behind the parity gate. Producer API
success, green counterexample tests or a static source gate cannot discharge
those tasks. No release, remote replication, host isolation, authority, guest
correctness or whole-system correctness is claimed.

## Final local checks at the blocked stop

All commands ran from the implementation worktree. The repository's pinned Nix
development shell supplied tools; task-owned executable `TMPDIR` and Cargo
targets were under `/dev/shm`, debug information and incremental compilation
were disabled, and Cargo used two build jobs. No source-path dependency or
producer source override was used.

| Executed command / surface | Observed result |
|---|---|
| `cargo test -p molten --lib -- wasm_component` after the final Clippy correction | 57 passed, zero failed or ignored. Includes 12 candidate parity/counterexample cases and seven published-materialization cases. |
| `cargo test -p molten --lib -- wasm_component:: wasm_performance:: system_extension:: hostcall world_replay` | 118 passed, zero failed or ignored. |
| `cargo test -p molten --lib -- wasm_executor wasm_preserves_abi` | 11 passed, zero failed or ignored. Selections overlap; do not sum them as distinct coverage. |
| `cargo test --workspace --lib` on the final source | Molten 1757 and Molten Core 629 passed; the other two library targets had zero cases. Zero failures or ignored cases. This is not the integration-target or optional-feature suite. |
| `cargo check --workspace --all-targets` | Passed. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed after replacing the candidate's manual modulo alignment check with `is_multiple_of`; no lint allowance or policy relaxation. |
| `cargo fmt --all -- --check`; `nixfmt --check flake.nix` | Passed. |
| Pinned workspace and `-p molten -- --lib` Octet producers | Both `clean`, zero findings, warnings or errors. |
| Actual CLI artifact import, `strict-ci` gate and remediation plan | Passed with the final complete corpus inventory below. |
| Inherited Tracey debt guard | Passed: 2966 requirements, 1032 referenced, 1934 unchanged baseline entries, zero dangling refs and five pending references. Pending references are identity resolution, not accepted coverage. |
| Strict native Cairn validation through the sibling provider and explicit provider policy | Passed with no findings/issues; policy BLAKE3 `7a67802377005c18ebded42899e7f564f5d87087b6c14ca70464b35886f340a9`. Package structure is not adoption acceptance. |
| Nix `release-dependency-profile` and `git-source-hash-binding` checks | Passed. The first also ran the positive/negative archive-path fixtures. Full `nix flake check`, VM integration and release promotion were not run. |

The final public-intake smoke executed a real temporary example against
`VerifiedBundle::load`, independently of the test harness. Malformed JSON,
the unmodified producer fixture's profile mismatch and stage reuse each returned
`component-admission-denial`. Owned failed children were absent afterward and
the caller's `occupied/keep` bytes remained `preserved`. Its exact output was:

```text
malformed=component-admission-denial producer-mismatch=component-admission-denial reused-stage=component-admission-denial
cleanup=owned-only producer-fixture=unmodified matching-production-bundle=false adoption=false
```

The executable and its owned scratch were removed. This proves those public
deny/cleanup paths, not successful production materialization or a consumer
cutover. Permanent tests preserve the behavioral resource, staging, receipt,
cancellation and replay boundaries; they do not turn the observed mismatches
into acceptance.

### Source-gate evidence and retained unsuccessful attempts

Emitted files are retained in
[`2026-10-26-runtime-source-gate/`](2026-10-26-runtime-source-gate/).
The workspace/library command, status, summary and provenance artifacts, full
corpus and actual canonical CLI receipts are retained, not recreated from prose.
Byte identities were checked when moving/copying them from executable scratch.

An initial directory-only corpus command omitted include-only facade paths from
its replay inventory. Artifact import and `strict-ci` correctly denied it:
critical-path/source-scope coverage failed and no valid fingerprint evidence
could be derived. The denied import and gate receipts remain beside the passing
ones. The gate was not weakened. The corrected command explicitly selects the
six required facade paths as well as the recursive source directory:

```sh
cargo octet object corpus receipt --output "$ARTIFACTS/object-corpus-receipt.json" \
  src src/job/dag.rs src/main.rs src/node/daemon.rs src/node/runtime.rs \
  src/octet/gate.rs src/upgrades/mod.rs
```

Earlier workspace/library `warning-only` summaries are also retained. Their
function-length findings were fixed by separating feature-policy admission from
resource translation and extracting declared-member admission; warning-only
output was never treated as acceptance. The final configured producers use the
unchanged catalogued Octet toolchain `nightly-2026-03-21`.

Final canonical CLI identities:

```text
artifact import: blake3:5b67813eae8be9b6d14e1489ec98a83f3dd062155b25fee2f5c6739886815026
strict-ci gate: blake3:63ad65533d091bafdfbe283d49bcd1c6e6a913cea00cfa8d83bf6b2a788d5176
remediation plan: blake3:4ca31d24298a5d8ceeea28511b5ab8ee46295f1efeef46a1d4f91ee3f9bff5a9
corpus file bytes: 3392a738d223d94a36509eee7153336c8214614b0f8d473571cc9f3dbe76df45
corpus object-set hash: b3:ed20db1fedf3821855ead151dcf4cb6120c385f3dafd4d5d1c142ea2ee6ddd06
```

The corpus contains 14992 function objects, all marked pure-cache-blocked.
It is source-token identity/review evidence, not a behavioral equivalence or
correctness proof. A passing static gate does not supersede the runtime
counterexamples or close the consumer tasks.

### Final executable-source identities

BLAKE3 over the checked files:

```text
5a87007c8c3b613efaaddd57a27dd4631c81fd031eb28a85703c37e84a21c10a  Cargo.lock
036a7eeb7b54e791368d48da64f54ca7158189b5feac7a77f004b7c7fd6f5f8b  src/wasm/component/runtime.rs
fde2d46891289eaa77bc6e25abcc819c5629c46e094fd7853b0998729694b13a  src/wasm/component/runtime/shared/mod.rs
892e9d2e2e8a44bc6b929dc305e99c41d1222cc210e8ac96df604d19250e997f  src/wasm/component/runtime/shared/configuration.rs
7593078b9ded2897d5bdffc32e2f37eb5491002910abb69105e149d5d889ab3d  src/wasm/component/runtime/shared/sliced.rs
aa7970ce8c542aef29a4124cfd4f59c4e76bcce1f16e85269ab2bb5bdcca5c4f  src/wasm/component/tests/parity/mod.rs
222ded35c7bac8756954c4124b506fa6dda0f1246d04c41ebf711ee569dfeec0  src/wasm/component/tests/parity/fixtures.rs
6e51b14f93c19157f57c8de7349edc8e5df25ece2ca161c7fe7e466ddf04503f  src/wasm/component/tests/parity/rejection.rs
02610f14e7a36ac9cbd68bf86daa3d0b5ef790002c463e85e4c66cb496998413  src/wasm/component/evidence/materialization/published.rs
db372382d9c0c22df2a18309f497e3747fd585bfffdde2ac96529ccbdb6a3c40  src/wasm/component/evidence/materialization/published/measurement.rs
10b890415707bdf7a7b26b2835844fab4cbbf18ed09d91233c9b35f52222268d  src/wasm/component/evidence/materialization/published/staging.rs
0edad1837b4ec1a6d3fd2d9d8c6286c9f6b41a84a481286323c8b4e5bfbc6728  src/wasm/component/evidence/materialization/published/tests.rs
```

The profile and WIT identities recorded in the preflight still match. Resuming
requires a published execution boundary that preserves the instantiation/callback
checkpoint and a reviewed per-memory/aggregate translation, followed by the
frozen dual-run corpus. The complete product effect world, actual actor and
extension composition, matching producer member trees, operated deadlines and
coupled cutover/rollback must still be exercised before adoption can close.
