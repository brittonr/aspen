# Naming review and exemption counterexamples

## Current findings

Historical run16 reports 18 path_segment_repetition findings on its pinned source. They follow the lint's current repeated-word rule; this review does not classify them as implementation false positives.

Fourteen are public identities: MoltenError; LocalStoreKind, LocalStorePath, LocalStoreEntryKind, LocalStoreEntry, LocalStoreRoot; NodeStateNamespaceKind, NodeStateEntryKind, NodeStateFileObservation, NodeStateFile, NodeStateRoot, NodeStatePath, NodeStateNamespace, NodeStateEntry.
Four were non-public: validate_local_locator, local_store_entry_kind, NodeStateInner, validate_node_state_locator.
Public compatibility needs a deliberate contract decision, not mechanical renaming or marker-looking prose. No public naming change or compatibility exemption was added to product code.

## Private source-only correction

The four private names are now `validate_locator` (in each of the two modules), `entry_kind`, and `RootDirectory`, with every call site updated. The shared private remote/content-locator recognition helper, `crate::locator::is_remote`, was already present; both validators still use it before their boundary-specific checks. This does not authorize startup or reclassify the historical run16 findings as a passing canonical source gate.

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

## Other remaining findings

The shared private remote/content-locator predicate is already in use; its focused locator tests passed. Boundary-specific validation order and errors remain at the original call sites. The two input validators should not gain panic assertions merely to satisfy assertion density.
The other assertion sites and the oversized local-store module still need responsibility/invariant review; no meaningless assertions or whitespace changes are justified.

No Octet naming-lint repair, exception promotion, complete source-gate pass, approved runtime cohort, or startup authority is established here.
