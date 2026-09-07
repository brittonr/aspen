# Naming review and exemption counterexamples

## Current findings

Run16 reports 18 path_segment_repetition findings. They follow the lint's current repeated-word rule; this review does not classify them as implementation false positives.

Fourteen are public identities: MoltenError; LocalStoreKind, LocalStorePath, LocalStoreEntryKind, LocalStoreEntry, LocalStoreRoot; NodeStateNamespaceKind, NodeStateEntryKind, NodeStateFileObservation, NodeStateFile, NodeStateRoot, NodeStatePath, NodeStateNamespace, NodeStateEntry.
Four are non-public: validate_local_locator, local_store_entry_kind, NodeStateInner, validate_node_state_locator.
Public compatibility needs a deliberate contract decision, not mechanical renaming or marker-looking prose. No naming change or new compatibility exemption was added to product code.

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

Both compound conditions repeat the same pure remote/content-locator recognition rule.
A shared private predicate can preserve contains-first and ordered prefix recognition while leaving boundary-specific validation order and errors at their existing call sites.
The two input validators should not gain panic assertions merely to satisfy assertion density.
The other assertion sites and the oversized local-store module still need responsibility/invariant review; no meaningless assertions or whitespace changes are justified.

No naming-lint repair, exception promotion, clean workspace, approved runtime cohort, or startup authority is established here.
