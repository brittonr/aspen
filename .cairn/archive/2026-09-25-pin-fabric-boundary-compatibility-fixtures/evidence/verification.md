# Verification: pin the fabric boundary compatibility fixtures

Base: `d825dc14b`. Builds ran under `nice -n 10` with `CARGO_BUILD_JOBS=16`, Nix `--max-jobs 2`, and a private
`CARGO_TARGET_DIR`.

## Pre-migration generation

- Commit: `3de348149^` = `59aa8d153548bca4242f76bceb3245b0a2374c39`, the parent of the fabric port/adapter
  migration `3de348149bc828db6e79c93c72f5b2e6525b2442`.
- Generator: `evidence/premigration-generator.rs`, added as `tests/fabric_boundary_fixture_generator.rs` in a
  throwaway detached worktree at that commit, alongside a copy of `tests/fabricboundarycompat/{cases,inputs,ports}.rs`.
  Command: `nix develop -c env FABRIC_BOUNDARY_FIXTURE_OUT=<dir> cargo test --test fabric_boundary_fixture_generator`,
  run under that commit's own flake and toolchain (`nightly-2026-05-26`). Result: 1 passed, 0 warnings.
- The generator also asserts `content_ref_from_bytes(canonical bytes) == projection ref` for each case.
- Inputs, compiled verbatim at both commits, with identical BLAKE3 in the throwaway worktree and in this change:

| File | BLAKE3 at `3de348149^` and at head |
| --- | --- |
| `tests/fabricboundarycompat/cases.rs` | `7efcc5a09d09017980a5b40c91122a80889177864c756472f0d3a480c9a794dd` |
| `tests/fabricboundarycompat/inputs.rs` | `2d05dce0c040561c9e54f36e6bb9009b816888a9b845687cdb3d37e80d6961e8` |
| `tests/fabricboundarycompat/ports.rs` | `f6655c8a2c9d1288e882900e2d2142b76adee00a108e69a5a87b9a86852043c8` |
| generator (`evidence/premigration-generator.rs`) | `90ffb80b282b453427c4867540515d945aef66fa65a29f27b9674680132736fc` |

- `evidence/fixture-inputs.b3` lists the BLAKE3 of every fixture, the manifest, the three input files, and the
  generator.
- Edits to the throwaway tree: none to tracked files. The only additions were the untracked generator and input
  files above. Generation ran four times in fresh throwaway worktrees at the same commit, and every run produced
  fixtures byte-identical to the checked-in ones (`diff -r` clean). The runs were:
  1. The first draft, a single `inputs.rs`. Before its first copy into the throwaway tree, one `sed` corrected two
     enum variant names that exist at neither commit (`TransportAdapterKind::Deterministic` and
     `DurableAdapterKind::Deterministic` became `::DeterministicSimulation`).
  2. After a whitespace-only `cargo fmt`.
  3. After the Octet repair split the inputs into `inputs.rs`, `ports.rs`, and `cases.rs`, qualified every owner path,
     and moved failures to `TestResult`/`OrFail`.
  4. After renaming the projection builders from `*_cases` to `*_projections`.
  The input values never changed; only structure and names did.
- `refs.tsv` (generator output, checked in at `tests/fixtures/fabric-boundary/refs.tsv`) lists name, ref, and byte
  length, and is mirrored in the table below.

| Fixture | Ref | Bytes |
| --- | --- | --- |
| membership-profile | `blake3:ffe8a49e…5a57` | 788 |
| membership-view | `blake3:356b8467…004f` | 2457 |
| assignment-transition | `blake3:1df4dbea…7431` | 1252 |
| time-profile | `blake3:19826005…83e2` | 1163 |
| transport-profile | `blake3:0d08225d…8c8e` | 1134 |
| transport-transition | `blake3:3b3d3f04…1964` | 870 |
| durable-profile | `blake3:224e7ff9…2e65` | 1031 |
| durable-transition | `blake3:5e06dc55…b584` | 825 |

## Tests at head

`cargo test --test fabricboundarycompat`: 5 passed. Every test returns `TestResult`, and failures carry `OrFail` labels.
The test code uses no `expect`, no `unwrap`, and no lint allows.

- `equal_explicit_inputs_reproduce_the_pre_migration_canonical_fixtures`: every live projection's canonical bytes
  and ref equal the fixture and `refs.tsv`, and a strict decode of each fixture under its pinned ref equals the live
  value.
- `one_field_membership_mutations_change_the_projection_refs`,
  `one_field_time_and_transport_mutations_change_the_projection_refs`, and
  `one_field_durability_mutations_change_the_projection_refs`: all eight refs move under a one-field mutation. The
  first run failed on the transport transition, because the mutated field (the protocol `version`) is not part of
  the transition evidence record. That is a real coverage finding. The mutation now changes `protocol_id`, which the
  event record binds.
- `tampered_or_truncated_fixtures_are_rejected`: a flipped last byte and a one-byte truncation both fail
  `strict_canonical_decode_with_ref` for all eight fixtures.

## Octet

Pinned `cargo octet check` (`cargo-octet 0.1.0`, `nightly-2026-03-21`, profile `workspace-metadata`). The base ran in
a clean worktree at `d825dc14b` and the change ran at head, each with its own `CARGO_TARGET_DIR` and artifact
directory:

```sh
cargo octet check --artifact-dir <dir>/root
cargo octet check -p molten --artifact-dir <dir>/lib -- --lib
```

| Scope | Base `d825dc14b` | Change | Delta |
| --- | --- | --- | --- |
| root (workspace) | 3562 | 3562 | 0 |
| `-p molten --lib` | 1493 | 1493 | 0 |

Every lint family has a delta of 0 in both scopes, and the per-(lint, crate, file) counts are identical, with paths
normalized and line numbers dropped. Before the repair, the first commit's test files added 41 root findings:
- 36 `non_trait_imports`;
- 2 `function_length` (the mutation test and `canonical_cases`);
- 1 `excessive_file_length` (the single 427-line `inputs.rs`);
- 1 `unbounded_collection_growth` (map growth while parsing `refs.tsv`);
- 1 `underscore_in_module_filename`.
Its test-level `#![allow(tigerstyle::no_unwrap)]` also hid the `expect` calls from the count.

The repair, with no allows:
- qualified owner paths everywhere, with only the `OrFail` trait imported;
- `TestResult`/`OrFail` in place of every `expect`;
- per-family mutation tests and per-family projection helpers;
- three input files of 155–167 lines each;
- `refs.tsv` parsed row by row without collection growth;
- the test binary renamed `fabricboundarycompat`.

Two side effects also had to be undone:
- Octet's `path_segment_repetition` exempts any item whose immediately preceding comment line contains
  "compatibility". The new `r[impl …compatibility]` marker placed directly above each projection had therefore hidden
  14 existing root findings (7 in lib). The marker now opens each marker block, so those findings are counted again.
- Projection builders named `*_cases` repeated their module name `cases`; they are now `*_projections`.

## Checks

- `cargo fmt --check`: exit 0 (rerun after the Octet repair).
- `cargo clippy --all-targets -- -D warnings`: exit 0 (rerun after the Octet repair).
- `cargo test --workspace` (default features), run on the first revision: exit 0, 2074 passed, 0 failed, 0 ignored.
  The Octet repair changed only the new test files and the position of the comment markers. It was not rerun; the
  focused test binary was rerun (5 passed). The
  `nativesystemextension` binary took 372.5 s: its test `native_executor_fails_closed_for_malformed_nonzero_timeout_flood_spawn_and_cancellation`
  kept one thread busy for about 6 minutes on a loaded host, then passed. That test predates this change and is unrelated to it.
- `nix build .#checks.x86_64-linux.inherited-tracey-debt`: exit 1. It still reports `dangling=18`, all from the
  unsynced F01/F09/F10/F12/chaoscontrol changes, none from this change. Counts moved from `referenced=864
  uncovered=1927` to `referenced=866 uncovered=1925`. The guard stops at dangling before it prints
  `unexpected_missing`, so a measurement-only copy of the guard with that early return removed was also run. It
  reports unexpected_missing moving from 3 to 1; the only remaining id is
  `molten.authority.nominal_references.octet.guard`. After the Octet repair, the measurement-only guard reports the
  same counts: `referenced=866 uncovered=1925 dangling=18`, with one `unexpected_missing` id.

## Claim boundary

The fixtures pin eight projections at the pre-migration commit and prove that they still hold at head for equal
inputs. They do not cover other fabric projections, live adapter behavior beyond these shared projections, or
release eligibility.
