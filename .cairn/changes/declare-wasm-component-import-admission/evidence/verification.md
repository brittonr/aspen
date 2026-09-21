# Verification: Wasm component import-table admission

## Baseline

Before implementation, these commands passed on the drain worktree (shared
`CARGO_TARGET_DIR`; per-worktree local target used after a sibling-worktree
binary collision was diagnosed):

- `cargo test -p molten --lib wasm_component`: 19 tests passed (admission,
  materialization, profile, receipt, shell suites).

The pre-change rail already denied ambient WASI imports
(`WASI_IMPORT_PREFIX` checks in `plan_component_execution`) but admitted no
explicit declared import table, world-surface match, or tool/verifier-bound
receipt.

## Implementation evidence

The change adds a focused pure family plus shell and plan wiring:

- `src/wasm/component/imports/mod.rs` and `imports/{surface,observation,receipt}.rs`:
  - typed `DeclaredManifest` (complete declared import table) built by
    `build_manifest` with canonical Preserves identity;
  - `DeclaredWorld` pinned by `declared_world(profile)` from
    the supported `molten.wasm.component.v1` profile and WIT source ref;
  - `DeclaredObservation` carrying extraction tool, verifier,
    component ref, world, imports, and exports;
  - pure `admit_declared` performing manifest-shape validation, exact
    WIT world matching, import-set and export-set comparison, ambient WASI
    denial, tool/verifier identity pinning, and manifest integrity recompute;
  - `observe_artifact_surface` extracting component import/export names from
    exact artifact bytes with `wasmparser` component sections (no compile, no
    instantiation), recording the pinned tool and verifier identities;
  - `record_surface_admission` host rail returning observation, verdict, and
    canonical receipt without instantiation; a missing manifest source fails
    closed;
  - `build_admission_evidence`/`validate_admission_evidence`
    with bounded fields, blocker count, and payload-size bounds that reject
    raw-bytes packing, plus `validate_non_claims` rejecting
    sandbox/correctness/guest-safety/release overclaim labels.
- `src/wasm/component/admission.rs`: `plan_component_execution` now requires
  the declared manifest and denies manifest/world/facts drift before
  instantiation.
- `src/wasm/component/runtime/shell.rs`: `ComponentExecutionRequest` carries
  `import_manifest`; execution admits the byte-derived observed surface before
  planning and instantiation.
- `src/wasm/component/tests/imports/mod.rs`: import-admission fixtures. The
  family uses `imports/mod.rs` and `tests/imports/mod.rs` directory layouts so
  the `component` and `component/tests` directories stay within the Octet
  module-file bound.

## Checks

These checks passed after implementation:

- `cargo fmt --all` then `cargo fmt --check`.
- `cargo test -p molten --lib -- wasm_component`: 26 tests passed (19 pre-change
  plus 7 new import-admission fixtures).
- `cargo check --workspace --all-targets`.
- `cargo clippy --workspace --all-targets -- -D warnings`.
- `cargo test -p molten --lib`: 1465 passed.
- `cargo octet check --artifact-dir target/octet-wasm-2` (workspace scope):
  3619 findings, 0 errors, `warning-only`. The pre-change checkout at
  `38cb87acd` reports 3619 findings with the same tool and toolchain, so this
  change adds no finding. Reaching the baseline required the reviewed renames in
  the family (no `component`/`import`/file-stem repetition), explicit
  `with_capacity` reservations in the observed-surface and blocker loops, and
  the directory layouts above; the first revision of the change added 57
  findings (`path_segment_repetition`, `unbounded_collection_growth`,
  `module_file_count`).
- Cairn: `validate --root . --strict` valid; `gate proposal|design|tasks`
  valid for this change.

Not run: Nix checks (see caveats).

## Traceability

- `r[aspen.wasm_import_admission.manifest]`: manifest builder/shape tests and
  missing/unsorted/tampered-manifest denials
  (`tests/imports/mod.rs`, `imports/surface.rs`).
- `r[aspen.wasm_import_admission.surface]`: world-surface and import-set
  comparison with drifting-world, undeclared-import, and ambient-WASI
  denials; execution denial before instantiation.
- `r[aspen.wasm_import_admission.admission]`: pure `admit_declared`
  determinism replay and tool-drift denial; shell byte-derived observation.
- `r[aspen.wasm_import_admission.evidence]`: receipt builder/validator with
  tool/verifier/manifest/world/import-set bindings, size-bounded payload
  rejection, and tamper rejection.
- `r[aspen.wasm_import_admission.nonclaims]`: overclaim-label rejection with
  unchanged fixed non-claim list.
- `r[aspen.wasm_import_admission.fixtures]`: positive exact-manifest/world/
  import-set fixtures plus negative drifting-world, undeclared-import,
  tool-drift, overclaim, and malformed-receipt fixtures.

## Broader check caveats

- Nix checks: the repository Nix flake evaluates against the shared
  `CARGO_TARGET_DIR` layout and was not exercised for this change; focused
  Cargo gates cover the changed surface. No Nix check regressed by this
  change's source edits (no dependency or lockfile change).
- The extraction tool identity is pinned to the profile constant
  `COMPONENT_WASMPARSER_VERSION` (`0.240.0`), which is the crate's direct
  `wasmparser = "0.240"` dependency (`Cargo.lock` resolves `0.240.0` for
  `molten`); the other locked `wasmparser` versions belong to transitive
  tooling. The pin is cohort-bound: a later `wasmparser` bump must update the
  constant, the admitted profile, and its fixtures together.

## Claim boundary

The evidence proves only that the recorded declared-surface admission
decides deterministically over the supplied typed facts and exact artifact
bytes for this source and profile cohort. It does not prove sandbox
completeness, component correctness, semantic equivalence, guest safety,
production readiness, or release eligibility.
