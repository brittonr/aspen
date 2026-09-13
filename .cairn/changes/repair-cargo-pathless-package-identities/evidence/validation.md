# Cargo repair evidence

## Result and boundary

The Nix-built Cargo removes the observed package-ID formatter panic and supports explicit pathless package identities.
Actual locked all-feature Molten metadata passes through the selected repository toolchain.
Nextest reaches test execution, but the default and CI profiles fail their recorded time limits.
Full Nix rejects six unresolved Tracey references. This change is not accepted or archived.

Base commit: `20e1233f82e42f821913408e8bc6497bab7942b7`.
Implementation branch: `completion/cargo-pathless-package-id-20260913`.
Cargo source: `rust-lang/cargo` at `4d1f984518c77fad6eeef4f40153b002a659e662`.
Patched tool description: `molten-pathless-package-id`.
Rustc remains `31a9463c6e2794a59ce57a8f37abc6966afc2a58`, with LLVM 22.1.6.

Cargo owns the schema decisions. Molten's toolchain maintainers own the consumer patch and its Nix selection.
Nix owns retrieval, patch application, compilation, and composition.
The patch adds no port, runtime dependency, metadata postprocessor, fabricated URL path, or replacement transport.
Upstream MIT, Apache, and third-party notices remain in the installed package.
The former explicit-name rejection was a compatibility barrier, not a proven violation of Cargo's prior contract.

## Executed checks

| Check | Result |
| --- | --- |
| Original schema suite | 15 unit tests and one doc test pass |
| Added regressions before correction | Four intended failures and 16 passes |
| Corrected schema suite | 20 unit tests and one doc test pass |
| Patched-source hash controls | Correct files pass. Unpatched input rejects |
| Nix Cargo build and schema check | Pass. The same 20 tests and doc test pass inside Nix |
| Tool selection | Pass for Cargo, rustc, rustfmt, both Clippy executables, and the complete Rust library tree |
| Actual locked all-feature metadata | Exit 0. Complete JSON read-back finds 769 packages and four Radicle identities |
| Default nextest, first round | Exit 124 during compilation. No test summary |
| Default nextest, warm round | Exit 124. 1,563 pass, one times out, two receive SIGTERM, and 80 do not run |
| Existing CI nextest profile | Exit 100. All 1,646 tests run. 1,645 pass and one times out |
| Workspace/all-target Clippy | Pass with `-D warnings` |
| Schema/all-target/all-feature Clippy | Pass with `-D warnings` |
| Rust and Nix formatting | Pass |
| Corrected strict Cairn validation and tasks gate | Pass for 56 active changes. No lifecycle acceptance |
| Full Nix | Exit 1 at inherited Tracey debt after 13 guard tests pass |
| Independent source review, second route | Exit 0. No concrete defect within 14 source reads/searches |

The schema controls cover explicit and inferred names, optional versions, Git references, JSON round-trips, ordinary URLs, and malformed inputs.
Missing inferred names, empty names/fragments/versions, forbidden queries, unsupported protocols, and invalid `path+` schemes reject.
The parser now reaches the applicable name/version validation for some inputs that previously failed the eager path check.
No universal diagnostic-precedence claim is made.

## Nix package and repeatability

Package: `/nix/store/gcimdyzhrk0696qh554n1rlmpfyhh2rm-molten-cargo-pathless-1.98.0-nightly`.
Derivation: `/nix/store/iykx4bqma0yqp8r9csjq36x62sjjxsav-molten-cargo-pathless-1.98.0-nightly.drv`.
A final Nix evaluation confirms that the expanded selection guard leaves this Cargo derivation unchanged.
The archive retains the binary BLAKE3 digest, derivation, closure, verbose versions, source manifests, and failed build attempts.

Nix generated the added `cargo-src` lock entry. Existing producer inputs remain unchanged.
`Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `.cargo/config.toml`, and `.config/nextest.toml` retain their base contents.
The package recipe verifies the exact patched Rust files before compilation.
It changes only the registry download route to the endpoint declared by crates.io.
Registry identities, versions, and locked SHA-256 checksums remain intact. New Molten evidence uses BLAKE3.

The first download route returned HTTP 403. A rejected alternative produced a duplicate default-registry alias.
The first corrected cold build reached its eight-minute limit during dependency compilation.
A separately recorded 24-minute package-build allocation passed with two build jobs.
Cargo reported 11m 04s for the release build and 59.41s for the schema check build.
All other check commands retained eight-minute limits. No test profile or runtime timeout changed.
These times do not establish performance gains.

## Actual consumer observations

The metadata command used `--locked --all-features --filter-platform x86_64-unknown-linux-gnu --format-version=1`.
Its complete output retains these package names:

- `executable-extent-conformance`
- `executable-extent-core`
- `executable-extent-linux`
- `vm-cohort-core`

The source revisions remain `025d9636f0161777710dac37b3c210ca0ad9483f` and `31f1696ba9391bfda8577a58af84f72361d5573e`.
The package IDs retain their Radicle URLs and explicit names. No false path component appears.

The default warm run spent 3m 25s in compilation and 266.659s in test execution.
`materializing_native_host_rejects_missing_mismatched_and_oversized_effect_values_without_retry` exceeded its 60-second limit.
The outer deadline terminated two other cases and left 80 tests unrun.
The CI run used the existing `ci` profile, not a modified timeout or a replacement result for the default profile.
It completed in 411.726s, but `native_executor_fails_closed_for_malformed_nonzero_timeout_flood_spawn_and_cancellation` exceeded its 180-second limit.
The archive retains the CI JUnit report and both failed outcomes. The cause of these durations remains unproven.

## Review and remaining gates

The first reviewer failed at the provider quota before any review.
The second route loaded the existing multi-account provider extension and the workspace and subagent authority guards.
One read-only reviewer completed two correlated source passes within a five-minute round.
The reviewer found no concrete compatibility or selection defect in the examined source.
Broader fragment combinations and Git-reference escaping remain unexamined.
The coordinator compared the cited source with the patch and Nix selections.
This is an advisory source review, not an executed test, provenance proof, security proof, or lifecycle approval.

Strict Cairn validation first rejected six tasks without concurrency markers and known requirement references.
The corrected tasks use `[serial]` and the known `r[...]` references.
The later validation and tasks gate pass under `legacy_default`, without an installation receipt.
Checked tasks cite bounded execution evidence. The last two tasks remain open.

The full Nix guard reports 2,781 requirements, 791 referenced, 1,990 uncovered, 1,924 baseline entries, and six dangling references.
Its terminal diagnostic is `error: dangling traceability references are not permitted`.
Cache errors precede that source-gate rejection. They are not its terminal cause.
The full command preceded Git enrollment of this new lifecycle package and the final selection-guard expansion.
Later direct Cairn and selection checks cover those surfaces separately. No final-source full Nix pass is claimed.
The six references, historical debt, strict Octet, remaining feature/target/runtime checks, and Molten acceptance remain open.

## Evidence files

`inputs.b3` binds the final implementation, configuration, and lifecycle task inputs.
`experiment.tar.gz` retains the campaign report, review prompts, complete metadata, JUnit, source counterexample, logs, and direct exits.
`payloads.b3` binds this report, the input manifest, and the archive.
Historical cold-build manifests retain their original meaning. They do not bind the later guard edits.

The early schema tests used a command-local compiler-wrapper bypass to isolate the observed cache lock.
The actual metadata and later Clippy/nextest commands used no wrapper override.
No result proves that the earlier shared kache lock problem is resolved.
No lifecycle sync, archive, integration, push, deployment, or whole-Molten completion occurred in this repair.
