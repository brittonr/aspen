# Naming review and exemption counterexamples

## Current findings

Historical run16 reports 18 path_segment_repetition findings on its pinned source. They follow the lint's current repeated-word rule; this review does not classify them as implementation false positives.

Fourteen are public identities: MoltenError; LocalStoreKind, LocalStorePath, LocalStoreEntryKind, LocalStoreEntry, LocalStoreRoot; NodeStateNamespaceKind, NodeStateEntryKind, NodeStateFileObservation, NodeStateFile, NodeStateRoot, NodeStatePath, NodeStateNamespace, NodeStateEntry.
Four were non-public: validate_local_locator, local_store_entry_kind, NodeStateInner, validate_node_state_locator.
Public compatibility needs a deliberate contract decision, not mechanical renaming or marker-looking prose. No public naming change or compatibility exemption was added to product code.

## Private source-only correction

The four private names are now `validate_locator` (local store), `validate_input` (node state, where the module is named `locator`), `entry_kind`, and `RootDirectory`, with every call site updated. The shared private remote/content-locator recognition helper, `crate::locator::is_remote`, was already present; both validators still use it before their boundary-specific checks. This does not authorize startup or reclassify the historical run16 findings as a passing canonical source gate.

Focused verification from the producer worktree: `cargo test --locked --offline -p molten-node-host` passed 25 tests; `cargo clippy --locked --offline -p molten-node-host --all-targets -- -D warnings` and `git diff --check` exited 0. These used installed Rust/Cargo/Clippy 1.97.1, `RUSTC_BOOTSTRAP=1` for the existing `register_tool` feature, disabled Cargo wrappers, and the retained `molten-node97` target directory—not the production pinned nightly or an Octet full-source gate. A separate compiled-client smoke against the built host library accepted a local object path, rejected remote and parent-traversal local-store locators and a remote node-state locator, then wrote/listed/read/removed one entry in its own temporary registry namespace. The throwaway source, binary, and namespace were removed.

## Reproduced owner defects

Octet source reviewed at e4ecf888a0b419d0175d2fe1d748c24322ecce89:
`src/naming/path_segment_repetition.rs`.

`has_compatibility_documentation` scans the whole item source for compatibility words, including the substring api.
`is_generated_or_unrelated_fixture` treats any filename containing registry as generated, without establishing Cargo-registry provenance.

Task10371 retained four compiler-metadata probes against the unchanged h3z exact-marker library:

| Source | Exit | Meaning |
|---|---:|---|
| plain.rs | 101 | node_label repetition diagnosed |
| documented.rs | 0 | genuine compatibility-documentation control |
| body.rs | 0 | false clean: ordinary string rapid contains api |
| registry.rs | 0 | false clean: source is byte-identical to plain.rs but filename hides it |

Body text is not documentation. A local filename does not establish generated/dependency provenance.
These are false-negative counterexamples, not exemptions to adopt in Molten.
Inputs and tool hashes matched before/after; no ICE markers appeared.
The reproducer deliberately expects the old unsafe behavior and must fail under a corrected implementation.

Replay: `sh verify.sh OUT DRIVER LIBRARY COMPILER`, absolute inputs/fresh OUT. It tests only the naming lint and unknown-lint denial, not the canonical source gate.
Private evidence: `~/.local/state/onix/molten-node-vm/locator-review/naming-probes/`.

## Isolated Octet candidate review, not tool admission

The separate `octet-worktrees/naming-exemptions` checkout remains at `e4ecf88` with uncommitted changes; the pinned Octet package and Molten source-gate tool are unchanged. The initial candidate did not compile (`str == &str`, `.iter()` on `Split`); both type errors were corrected without replacing its other work. A compiled probe then exposed another false clean for a local `registry/` directory. The reviewed candidate recognizes only the `.cargo/registry/src` component sequence as a Cargo cache and retains the other existing generated-path rules. A documented name with an intervening `#[inline]` attribute initially produced an incorrect naming error; the reviewed candidate skips adjacent single-line attributes without treating their contents as documentation. The existing compatibility UI fixture now covers that case.

Pinned `nightly-2026-03-21` Cargo built this local library with `--locked --offline` after fetching the lockfile's exact Dylint revision. With the installed Dylint driver and `-D path_segment_repetition`, compiled-client probes produced:

| Input | Exit | Naming result |
|---|---:|---|
| plain.rs | 101 | repeated `node_label` diagnosed |
| documented.rs | 0 | genuine compatibility comment accepted |
| body.rs | 101 | body `"rapid"` cannot exempt |
| registry.rs | 101 | local filename cannot exempt |
| local registry/probe.rs | 101 | local directory cannot exempt |
| annotated compatibility comment | 0 | `#[inline]` does not hide adjacent documentation |
| Octet compatibility UI fixture | 0 | annotated compatibility still accepted |

`cargo test --locked --offline -p octet --lib naming::path_segment_repetition::tests` passed 2/2; Rustfmt check of both modified files and `git diff --check` passed. The wider `cargo test --locked --offline -p octet --lib` did **not** pass: 120 tests passed and eight catalog/manifest snapshot tests failed. This candidate therefore has no full-suite, canonical source-gate, release, or compatibility approval. Reviewed source BLAKE3: `5a35565d1a4ee86f3635d41da6d768ba9fb2be5fea481555cf28fb7944206aec`; compatibility fixture BLAKE3: `ec0438ea9e21647dbdb64768e2202dafcb81ffdc45cc070d44e1529b322b8ddc`; built local library BLAKE3: `478cc9fc660ad6e2135760f05505bbe67e3b38e12f0ba79a65b6a79e0a1f5884`.

## Source responsibility review after run16

Producer commits `b940703aa`, `86d3694a`, and `9bb4b648c0ce73a3591d9e42dbbcfd71e436f9bd` keep public host APIs intact. `local_store/mod.rs` now only reexports its existing API; `path.rs`, `root.rs`, and `typed.rs` own path admission, directory operations, and typed transitions separately (10, 131, 268, and 113 lines). Node-state entry enumeration and observed-file/directory handling live in `node/state/enumeration.rs` and `node/state/filesystem.rs`. The listing bound remains in the same iteration before `push`, and the directory/handle checks remain fallible.

That source-only stage passed 25 package tests, all-target Clippy with `-D warnings`, stable Rustfmt check of the changed host files (with existing nightly-only formatting options reported as ignored), and `git diff --check`. A separate compiled client accepted local writes/reads/lists/removal through both public storage APIs, checked an acquired regular-file observation, and rejected remote and traversal locators. Its throwaway source, executable, and namespace were removed. This used the installed Rust 1.97.1 tools with `RUSTC_BOOTSTRAP=1` and disabled Cargo wrappers; it did not substitute for the pinned gate or real startup.

The unchanged pinned Octet gate on source commit `9bb4b648c0ce73a3591d9e42dbbcfd71e436f9bd` still failed: exit 2/Cargo 101, **14 node-host errors**, all `path_segment_repetition` on the public identities listed above. No assertion-density, excessive-file-length, non-trait-import, too-many-parameters, or unbounded-collection-growth findings remain. The gate kept config hash `b3:e32044b5acf7c094834d696f44b79f56d5cac4daaa69f932de60126abc03488c` and profile hash `b3:0e99a5a4f5f1442b8c0244035b69d30c77eec2ce301286ef91cc50bbf60e8e20`. Exact invocation, exit status, output, and summary: `~/.local/state/onix/molten-node-vm/source-refactor-20260924-86d3694a-linked/{invocation-final.json,stderr-final.log,artifacts-final/summary.txt}`. Before the final run, a wrapper-enabled attempt stalled in `kache` and a wrapper-disabled attempt lacked a linker; both failed attempts were retained under sibling evidence roots. The first correctly linked source check on `86d3694a` reported 29 errors; private/source-structure corrections at `9bb4b648c` reduced them to the 14 public names.

These public names needed an explicit API-compatibility/naming-owner decision at that revision. They were not then mechanically renamed, given marker-like comments, suppressed, or relabeled as false positives. The uncommitted Octet candidate in the separate worktree remains unpromoted after its eight wider-suite failures. **No passing source gate or normal-node startup authority was claimed.**

## Approved breaking name cutover

The operator subsequently chose a breaking public API rename, rather than retaining the names or seeking a scoped lint exception. Source commit `cd6690493ce1ff9501415c0cebf17e5b2615dd5d` migrates every reachable producer caller, host declaration/re-export, facade test, structural rule fixture, and current contract without legacy aliases:

| Boundary | Removed names → current names |
| --- | --- |
| Error | `MoltenError` → `Failure` |
| Local store | `LocalStoreKind` → `Category`; `LocalStorePath` → `RelativeLocator`; `LocalStoreEntryKind` → `ObjectKind`; `LocalStoreEntry` → `StoredEntry`; `LocalStoreRoot` → `DirectoryHandle` |
| Node state | `NodeStateNamespaceKind` → `NamespaceKind`; `NodeStateEntryKind` → `EntryKind`; `NodeStateFileObservation` → `FileObservation`; `NodeStateFile` → `AcquiredFile`; `NodeStateRoot` → `Root`; `NodeStatePath` → `RelativePath`; `NodeStateNamespace` → `DirectoryView`; `NodeStateEntry` → `DirectoryEntry` |

The two facade paths expose the same new type identities; existing enum variants, diagnostics, directory layout, read bounds, capability semantics, and startup denial remain. A search across active Rust source, examples, tests, verification, rules, and checker code found no removed identifier; historical mutation patches, log excerpts, and frozen source-scoped designs remain identified as historical. This is a deliberate compile-time break for callers of the removed names, not a compatibility shim.

Installed Rust/Cargo/Clippy 1.97.1 with wrappers disabled and `RUSTC_BOOTSTRAP=1` passed 25 host tests, two root-facade tests, `cargo check --locked --offline -p molten --all-targets`, and all-target Clippy `-D warnings` for both the host and root producer. A separate compiled client exercised the new types through local-store write/read/list/remove, node-state root/namespace write/observe/list/read/remove, and traversal/remote rejection; its temporary files were removed. A structural root-reacquisition pattern matched its positive fixture and did not match the carried-authority negative fixture. Stable Rustfmt checked 14 host source files and eight host tests, with nightly-only options reported as ignored; `git diff --check` passed. The workspace-wide check with the available March nightly failed, including on untouched `molten-core` source; the repository's pinned nightly-2026-05-26 formatter was unavailable. The available stable formatter proposed layout changes in 529 of 711 changed Rust files, including many lines unrelated to the rename; no mass formatting or toolchain substitution was used.

The unchanged canonical Octet command on `cd6690493` **failed** with exit 2/Cargo 101 and **14,529 root `molten` errors, zero `molten_node_host` findings**. These include 4,649 assertion-density, 2,539 non-trait-import, and 1,806 path-segment-repetition findings across root source. Config and profile hashes match the earlier 14-host-finding run. That earlier run reported only the changed host crate; it did not establish a clean root. No lint scope, setting, severity, or tool package was changed. Exact command, tool inputs, status, raw stderr, and 1.6 MiB summary are retained in `~/.local/state/onix/molten-node-vm/public-api-cutover-20260924/strict-cd6690493/`. Producer Cairn validation/gates also could not parse the existing generated policy (`initial workflow profile is missing: outcome-machine`); no Cairn pass is claimed.

The rename resolves the 14 diagnosed host identities but **does not clear the canonical source gate, approve the normal-node startup cohort, authorize a listener/VM/replay, or admit a Cua service**.

## Root-gate scope triage

The 14,529-error summary indexes 7,952 distinct `(lint, normalized path, line)` locations across 892 paths; 6,517 location keys occur twice, consistent with repeated checks of root targets. Of the raw findings, 14,518 point under `src/`, ten at five `crates/molten-core/src/` filenames, and one at `Cargo.lock`. Subsystems include fabric consistency (1,227 raw), WASM (1,074), harness (970), and node (910); this is not a host-only or startup-only residue. The dominant unique-location counts are assertion density 2,430, non-trait imports 1,401, path-segment repetition 1,004, ambiguous parameters 804, and numeric units 673. These are observations of the retained `summary.txt` index, **not** an adjusted gate count.

Representative pre-existing source: `src/artifacts/parts/mod/p001/body.rs` reports excessive length (566 lines), density at line 18, and parameter ambiguity at lines 428 and 433; comparison of `9bb4b648c..cd6690493` for that file shows eight error-type identifier replacements only. `src/fabric_time/canonical.rs` reports 810-line length and non-trait imports at its unchanged opening lines; its diff similarly contains only error-type substitutions. The Octet `non_trait_imports` rule intentionally diagnoses non-trait private `use` items (`src/naming/non_trait_imports.rs`). The `excessive_file_length` rule scans every compiler source-map file without a `.rs` suffix check (`src/structure/excessive_file_length.rs` in the pinned Octet source), causing an apparent false positive on the 9,822-line `Cargo.lock`, which a root test includes with `include_str!` at `src/protocol/parts/session/tests/m000/p002/body.rs:212`. That single tool defect cannot explain the thousands of Rust-source findings.

`Cargo.toml` selects both `-p molten` and `-p molten-node-host` with `--all-targets`, and `specs/node-startup-evidence/spec.md` requires complete strict checks, not a warning budget or caller-asserted clean count. A corrected Octet file-kind rule alone will not pass this gate. Full root-source remediation is a separate broad migration; choosing a different measured source cohort or gate scope would require explicit policy/owner approval and corresponding spec updates, never silent path exclusion. Until an approved route passes the pinned gate and independent cohort selection, the unchanged `real-cohort-not-approved` startup denial remains binding.

## Historical remaining findings

The shared private remote/content-locator predicate is already in use; its focused locator tests passed. Boundary-specific validation order and errors remain at the original call sites. The two input validators should not gain panic assertions merely to satisfy assertion density.
The historical run16 assertion sites and oversized local-store module were subsequently split by responsibility as documented above; they are not findings in the pinned host-only result at `9bb4b648c` or in the host crate at `cd6690493`.

No published or pinned Octet naming-lint repair, exception promotion, complete source-gate pass, approved runtime cohort, or startup authority is established here.
