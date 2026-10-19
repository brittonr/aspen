# Admitted Raft placement member count preflight

## Public boundary and precedence

After `validate_group_id`, check `input.admitted_members.len()` against
the existing `MAX_RAFT_MEMBERS = 32` with the label
`"consensus placement admitted members"`. This constant-time check
precedes the generic `validate_refs` loops, `placement_diagnostics`,
report serialization, and `admitted_members.clone()`. A 33-member
input returns the existing `MoltenError::invalid_harness` count-error
shape rather than a decision report with per-member diagnostics.

Retain all existing `validate_refs` calls and their order for admitted
inputs. Exactly 32 canonical members with matching candidates and
sufficient fault domains still return a pass report; a malformed
candidate/admitted ref at 32 still fails with the original canonical
ref diagnostic. Existing duplicate membership behavior is unchanged:
this is a count guard, not new uniqueness validation.

Do not apply the 32-member bound to `candidate_members`, which are a
candidate pool, not the list of actual admitted group members. Their
existing generic `MAX_RAFT_REFS = 4,096` validation remains, as do
fault-domain, evidence, denied-candidate, and refresh ref contracts.
With at most 32 admitted entries, per-unrecognized-member diagnostics
are bounded below the existing `MAX_RAFT_DIAGNOSTICS = 256` limit.

## Verification

Run a failing-before test with 33 valid admitted members; demonstrate
pass at exactly 32, existing malformed-ref precedence and duplicate
behavior at 32, and a 33-entry candidate pool still eligible with
32 admitted. After repair, exercise focused Raft library tests and a
compiled public API caller. Mirror only this disjoint owner into the
composition and run scoped fmt, Clippy, library Octet and strict Cairn.
The warning-only library lint profile is not the unchanged full gate.
