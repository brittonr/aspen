# Private Raft message rejection

## Scope and ownership

The source parent is `6db5f15c4ab3fe1ccab9948875f3c7b145350701`. Eight private wrong-family fallbacks now return `MoltenError::InvalidHarness` instead of a panic. Current public dispatchers select only matching families. This is defensive source-policy work, not eight demonstrated public defects.

Four private encoders return `Result<IOValue>`. The public encoder already returns `Result`. Four transition helpers already return `Result<()>`. Each helper retains its two successful variants and explicitly rejects the six other variants. Valid record fields, order, handlers, and arguments remain unchanged.

The Molten consistency owner maintains these helpers and tests. The durable contribution is fixed pre-change wire coverage and rejection tests for every message family. Public APIs, parsing, protocol schemas, producer pins, profiles, authority rules, and accepted specifications remain unchanged. The accepted implementation boundary is `.cairn/specs/consensus/spec.md`, `molten.fabric_consistency.live_raft`.

## Baseline, capture, and red evidence

The unchanged Raft suite passed 46 tests. A test-only recorder captured ten fixtures from the old encoders. These fixtures cover all eight variants, empty and nonempty append batches, and empty and nonempty snapshot completed-request inventories. They do not represent arbitrary empty application snapshots.

The recorder required an explicit output root and refused existing directories and files. A second invocation rejected the existing directory with exit 101. BLAKE3 checks verified that all ten files remained unchanged. The recorder is absent from the retained source. Its source and outputs remain in `raft-pre-change-source.tar.gz` and `raft-pre-change-wire/` inside the experiment archive.

The red suite passed 49 tests and failed eight tests, exit 101. All 30 wrong-family encoder cases panicked. All 60 transition cases panicked across both initial persistence flags. The tests preserved complete transition state, existing ordered effects, and persistence flags while they caught the panics. Wire controls and direct-handler comparisons passed before the repair.

The encoder red tests required a non-panic result. The green tests add exact error assertions and a matching-family control. No dual old/new adapter remains. `raft-private-rejection-red.tar.gz` preserves the rejected source.

## Green evidence and policy corrections

The first green suite passed 58 tests. Both nextest profiles passed 1,660 tests with zero skipped. Workspace/all-target Clippy passed with `-D warnings`. The first strict receipt denied 6,817 warnings and 314 critical findings:

`blake3:74e1dd29166fca1de6cf56d0fd81d5ea511f26b2bf80356d9f2c7e8274e17b93`

That source introduced catch-all rejection arms and test-style findings. A follow-up used explicit rejected variants, qualified owner paths, and smaller oracle helpers. It preserved all test inputs and assertions. The second green suite again passed 58 tests and both 1,660-test nextest profiles. Clippy passed. Its strict receipt denied 6,795 warnings and 314 critical findings:

`blake3:cf45145d440f7682d471d8d825ab9aeb2621ba4f31656b15ea6f324baba10c8e`

The final source only renames the remaining test with a repeated owner name. `logs/raft-final-name-only.patch` records this one-line change. No assertion, body, fixture, selector profile, or product behavior changes in that step. The first and second green source archives and their artifacts remain separate.

The tests require exact private error variants and diagnostic text. Encoder inputs remain unchanged. Rejected dispatch preserves full state, existing effect order, and both persistence flags. Fixed bytes, decoded values, content references, and decoded envelopes match the old fixtures. Truncated fixtures reject without fixture mutation.

## Final checks

| Check | Direct exit | Result |
|---|---:|---|
| Focused Raft suite | 0 | 58 passed, zero failed or ignored |
| Default nextest | 0 | 1,660 passed, zero skipped, 12 binaries |
| CI nextest | 0 | 1,660 passed, zero skipped, 12 binaries |
| Workspace/all-target Clippy | 0 | `-D warnings` |
| Formatting and whitespace | 0 | Unchanged source after checks |
| Configured Octet | 0 | Warning-only, 6,794 warnings, zero errors |
| Complete `src` corpus | 0 | 1,368 input files, 13,711 objects, 1,171 object-bearing paths |
| Public artifact import | 0 | Complete current artifacts imported |
| Public `strict-ci` gate | 1 | Deny, 6,794 warnings and 314 critical findings |
| Full Nix | 1 | Thirteen Tracey tests passed, then six references rejected |
| Explicit-policy Cairn validation | 0 | 56 changes, no reported issues, no installation receipt |

The default nextest test phase took 56.580 seconds after a 1m34s build. CI took 56.728 seconds after a 0.65-second build. These are scoped observations, not performance claims. The actual CI JUnit file was copied and compared. Its BLAKE3 digest is `823b2f91b50fddb849bb4d63e799109bcec3e61d871d0a1d6265d6a470b87796`.

The final strict receipt is:

`blake3:84058b9004b9ed33a38e4342078cbea306851777cbff5f529a0941fe8251c03a`

Only `strict-status-clean` and `no-critical-findings` fail. Artifact, schema, scope, fingerprint, current metadata, and linkage checks pass. The critical families contain 216 `unbounded_collection_growth`, 80 `no_unwrap`, 17 `ambient_clock`, and one `no_panic` finding. The remaining panic finding is in an integration-test helper, not an established public runtime defect.

The final catalog contains no findings at the four new Rust test files. The 16 removed `no_panic` findings represent library/test duplication of the eight private fallbacks. Counts do not establish semantic correctness.

The complete sorted corpus invocation matches the receipt's replay command. Its object-set identity is `b3:9f2fa8a57ecee48e32835536c050ee25ed42cb6ca5086c420a7d819d36addb13`. Object-bearing paths are not the full input inventory. Dependency, effect, macro, and pure-cache caveats remain.

Full Nix reports 2,781 definitions, 791 referenced, 1,990 uncovered, 1,924 baseline entries, and six dangling references. These are the four F12 references and two ChaosControl references already blocked on owning lifecycle work. No definition, rejection rule, debt classification, or baseline changed. The root package input remains `./.`. The configuration-source filter excludes `target`, `.direnv`, and `.git`, not `.preserves` fixtures. This does not establish a complete Nix build.

Cairn used the immutable canonical policy snapshot at `/nix/store/69f8rrsv266dvpamj90b87wspv93832c-source`. Its validation receipt is `fa6775724cc703a8bde2a283c0c0cf3728b43e1ce2b4c2bcecdaaefae1942d8a`. Structural validation does not grant acceptance, sync, archive, or release authority.

## Review and claim limits

The coordinator checked the error constructor and the source diffs. Bounded readers examined the constructor, implementation, and policy follow-up. They found no change-induced defect within their scopes. These correlated source reviews are advisory, not independent approval.

Static diagnostics become owned strings in `InvalidHarness`. Display adds `invalid harness artifact: `. The constructor performs no application I/O or authority lookup. Allocation remains a runtime dependency. Allocation-failure safety and custom-allocator behavior are not established.

The positive transition oracle compares complete state, ordered effects, and persistence decisions. Both paths share production handlers, higher-term observation, and finalization. This establishes routing equivalence for selected cases, not independent handler correctness or a saved pre-change transition baseline. The tests do not independently preserve consumed sender and message values.

No new lifecycle requirement or accepted-spec promotion is part of this repair. All existing Cairn, strict Octet, feature, target, process, VM, fault, and release obligations remain. No baseline, suppression, policy relaxation, or synthetic clean status was added.

## Evidence and reproduction

`inputs.b3` binds selected build inputs, the complete Rust inventory under `src`, `crates`, `tests`, and `tools`, and all ten binary fixtures. `payloads.b3` binds this report, the input manifest, and `experiment.tar.gz`. The archive retains direct logs and exits, available task envelopes, complete Octet artifacts, ledgers, wire fixtures, source archives, reviews, and failed attempts.

The source commands use two Cargo jobs, two test threads, one Nix job, two cores, and eight-minute limits. They retain the normal wrapper and `/tmp/molten-completion-20260913-target`. Source archives use sorted tar entries and `gzip -n`. Full archive comparisons and BLAKE3 checks precede publication.

The public test commands are:

```text
cargo test --locked -p molten --lib fabric_consistency::raft::
cargo nextest run --locked --test-threads 2
cargo nextest run --locked --test-threads 2 --profile ci
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo octet check --artifact-dir ARTIFACTS
cargo octet object corpus receipt --output RECEIPT.json SORTED_SRC_FILES
cargo run --locked --bin molten -- test octet artifacts import --artifacts ARTIFACTS --ledger LEDGER
cargo run --locked --bin molten -- test octet gate --artifacts ARTIFACTS --profile strict-ci
nix flake check --no-write-lock-file --builders '' --max-jobs 1 --cores 2 -L
```

Initial recorder removal failed because its new file was staged. The next attempt removed that owned file, then failed because Git removed its empty parent. Both failures remain recorded. Creation of the wire-test source restored the directory before the fixture copy.

The later task lookup for 2189 identified unrelated work. The original focused log and zero exit survive. No replacement terminal receipt or rerun was used to hide that gap. Publication task 2082 also lacks its expected envelope. The separate `combined-publication-audit-followup.md` corrects the earlier report without changing its historical evidence.

The published parent, primary staging, and unrelated worktrees remain separate. This package does not establish mainline integration, deployment, production readiness, or whole-Molten completion.
