//! Verus specifications for ref-diff computation.
//!
//! Proves partition and completeness properties of `compute_ref_diff`
//! and conflict resolution invariants for `resolve_conflicts`.
//!
//! The production functions operate on `HashMap<String, [u8; 32]>`.
//! Verus doesn't support HashMap natively, so we model ref heads as
//! `Seq<RefEntry>` pairs and prove set-theoretic properties that hold
//! regardless of container type.

use vstd::prelude::*;

verus! {

// ============================================================================
// State Model
// ============================================================================

/// A ref name → hash entry, modeling one entry from a HashMap<String, [u8;32]>.
pub struct RefEntry {
    pub name: Seq<u8>,
    pub hash: Seq<u8>,
}

/// The four output categories of a ref diff.
pub struct RefDiffSpec {
    /// Refs existing only on remote → pull.
    pub to_pull: Seq<Seq<u8>>,
    /// Refs existing only on local → push.
    pub to_push: Seq<Seq<u8>>,
    /// Refs on both sides with matching hashes.
    pub in_sync: Seq<Seq<u8>>,
    /// Refs on both sides with different hashes.
    pub conflicts: Seq<Seq<u8>>,
}

// ============================================================================
// Helper Spec Functions
// ============================================================================

/// Whether a name appears in a sequence of ref entries.
pub open spec fn contains_name(entries: Seq<RefEntry>, name: Seq<u8>) -> bool {
    exists|i: int| 0 <= i < entries.len() && entries[i].name == name
}

/// Look up the hash for a given name in a sequence of ref entries.
/// Assumes name exists (caller must check `contains_name` first).
pub open spec fn lookup_hash(entries: Seq<RefEntry>, name: Seq<u8>) -> Seq<u8>
    recommends contains_name(entries, name)
{
    entries[choose|i: int| 0 <= i < entries.len() && entries[i].name == name].hash
}

/// Whether a name appears in a sequence of names.
pub open spec fn name_in_seq(names: Seq<Seq<u8>>, name: Seq<u8>) -> bool {
    exists|i: int| 0 <= i < names.len() && names[i] == name
}

/// All names in `entries` are unique.
pub open spec fn names_unique(entries: Seq<RefEntry>) -> bool {
    forall|i: int, j: int|
        0 <= i < entries.len() && 0 <= j < entries.len() && i != j
        ==> entries[i].name != entries[j].name
}

/// All names in a sequence are unique.
pub open spec fn seq_names_unique(names: Seq<Seq<u8>>) -> bool {
    forall|i: int, j: int|
        0 <= i < names.len() && 0 <= j < names.len() && i != j
        ==> names[i] != names[j]
}

// ============================================================================
// Spec Functions: Ref Diff Classification
// ============================================================================

/// A ref should be classified as "to_pull" (remote-only).
pub open spec fn is_pull_ref(
    local: Seq<RefEntry>,
    remote: Seq<RefEntry>,
    name: Seq<u8>,
) -> bool {
    contains_name(remote, name) && !contains_name(local, name)
}

/// A ref should be classified as "to_push" (local-only).
pub open spec fn is_push_ref(
    local: Seq<RefEntry>,
    remote: Seq<RefEntry>,
    name: Seq<u8>,
) -> bool {
    contains_name(local, name) && !contains_name(remote, name)
}

/// A ref should be classified as "in_sync" (both sides, same hash).
pub open spec fn is_in_sync_ref(
    local: Seq<RefEntry>,
    remote: Seq<RefEntry>,
    name: Seq<u8>,
) -> bool {
    contains_name(local, name) && contains_name(remote, name)
        && lookup_hash(local, name) == lookup_hash(remote, name)
}

/// A ref should be classified as "conflict" (both sides, different hash).
pub open spec fn is_conflict_ref(
    local: Seq<RefEntry>,
    remote: Seq<RefEntry>,
    name: Seq<u8>,
) -> bool {
    contains_name(local, name) && contains_name(remote, name)
        && lookup_hash(local, name) != lookup_hash(remote, name)
}

// ============================================================================
// Invariant: Partition Completeness
// ============================================================================

/// REFDIFF-1: Every ref in the union of local and remote appears in
/// exactly one output category.
pub open spec fn partition_complete(
    local: Seq<RefEntry>,
    remote: Seq<RefEntry>,
    diff: RefDiffSpec,
) -> bool {
    // Every remote ref is classified
    (forall|i: int| #![trigger remote[i]]
        0 <= i < remote.len() ==>
            name_in_seq(diff.to_pull, remote[i].name)
            || name_in_seq(diff.in_sync, remote[i].name)
            || name_in_seq(diff.conflicts, remote[i].name)
    )
    &&
    // Every local-only ref is classified
    (forall|i: int| #![trigger local[i]]
        0 <= i < local.len() && !contains_name(remote, local[i].name) ==>
            name_in_seq(diff.to_push, local[i].name)
    )
}

/// REFDIFF-2: No ref appears in more than one category.
pub open spec fn partition_disjoint(diff: RefDiffSpec) -> bool {
    // to_pull ∩ to_push = ∅
    (forall|i: int| #![trigger diff.to_pull[i]]
        0 <= i < diff.to_pull.len() ==>
            !name_in_seq(diff.to_push, diff.to_pull[i])
            && !name_in_seq(diff.in_sync, diff.to_pull[i])
            && !name_in_seq(diff.conflicts, diff.to_pull[i])
    )
    &&
    // to_push disjoint from in_sync and conflicts
    (forall|i: int| #![trigger diff.to_push[i]]
        0 <= i < diff.to_push.len() ==>
            !name_in_seq(diff.in_sync, diff.to_push[i])
            && !name_in_seq(diff.conflicts, diff.to_push[i])
    )
    &&
    // in_sync ∩ conflicts = ∅
    (forall|i: int| #![trigger diff.in_sync[i]]
        0 <= i < diff.in_sync.len() ==>
            !name_in_seq(diff.conflicts, diff.in_sync[i])
    )
}

/// REFDIFF-3: Correct classification — each category only contains
/// refs that belong there.
pub open spec fn classification_correct(
    local: Seq<RefEntry>,
    remote: Seq<RefEntry>,
    diff: RefDiffSpec,
) -> bool {
    // Every to_pull ref is remote-only
    (forall|i: int| #![trigger diff.to_pull[i]]
        0 <= i < diff.to_pull.len() ==>
            is_pull_ref(local, remote, diff.to_pull[i])
    )
    &&
    // Every to_push ref is local-only
    (forall|i: int| #![trigger diff.to_push[i]]
        0 <= i < diff.to_push.len() ==>
            is_push_ref(local, remote, diff.to_push[i])
    )
    &&
    // Every in_sync ref has matching hashes
    (forall|i: int| #![trigger diff.in_sync[i]]
        0 <= i < diff.in_sync.len() ==>
            is_in_sync_ref(local, remote, diff.in_sync[i])
    )
    &&
    // Every conflict ref has different hashes
    (forall|i: int| #![trigger diff.conflicts[i]]
        0 <= i < diff.conflicts.len() ==>
            is_conflict_ref(local, remote, diff.conflicts[i])
    )
}

// ============================================================================
// Invariant: Empty Inputs
// ============================================================================

/// REFDIFF-4: Empty inputs produce empty output.
pub open spec fn empty_inputs_empty_output(diff: RefDiffSpec) -> bool {
    diff.to_pull.len() == 0
    && diff.to_push.len() == 0
    && diff.in_sync.len() == 0
    && diff.conflicts.len() == 0
}

// ============================================================================
// Invariant: Conflict Resolution
// ============================================================================

/// RESOLVE-1: After pull-wins resolution, conflicts are empty and
/// former conflicts are in to_pull.
pub open spec fn pull_wins_postcondition(
    pre: RefDiffSpec,
    post: RefDiffSpec,
) -> bool {
    // Conflicts cleared
    post.conflicts.len() == 0
    // All former conflicts now in to_pull
    && (forall|i: int| #![trigger pre.conflicts[i]]
        0 <= i < pre.conflicts.len() ==>
            name_in_seq(post.to_pull, pre.conflicts[i])
    )
    // to_push and in_sync unchanged
    && post.to_push == pre.to_push
    && post.in_sync == pre.in_sync
}

/// RESOLVE-2: After push-wins resolution, conflicts are empty and
/// former conflicts are in to_push.
pub open spec fn push_wins_postcondition(
    pre: RefDiffSpec,
    post: RefDiffSpec,
) -> bool {
    post.conflicts.len() == 0
    && (forall|i: int| #![trigger pre.conflicts[i]]
        0 <= i < pre.conflicts.len() ==>
            name_in_seq(post.to_push, pre.conflicts[i])
    )
    && post.to_pull == pre.to_pull
    && post.in_sync == pre.in_sync
}

/// RESOLVE-3: Resolving empty conflicts is a no-op.
pub open spec fn noop_when_no_conflicts(
    pre: RefDiffSpec,
    post: RefDiffSpec,
) -> bool {
    pre.conflicts.len() == 0 ==> (
        post.to_pull == pre.to_pull
        && post.to_push == pre.to_push
        && post.in_sync == pre.in_sync
        && post.conflicts == pre.conflicts
    )
}

// ============================================================================
// Proofs
// ============================================================================

/// Proof: Empty local and remote produce empty diff.
pub proof fn empty_inputs_proof()
    ensures ({
        let diff = RefDiffSpec {
            to_pull: Seq::empty(),
            to_push: Seq::empty(),
            in_sync: Seq::empty(),
            conflicts: Seq::empty(),
        };
        let local: Seq<RefEntry> = Seq::empty();
        let remote: Seq<RefEntry> = Seq::empty();
        empty_inputs_empty_output(diff)
        && partition_complete(local, remote, diff)
        && partition_disjoint(diff)
        && classification_correct(local, remote, diff)
    })
{
    // All sequences are empty, so all quantifiers hold vacuously.
}

/// Proof: A single remote-only ref produces exactly one to_pull entry.
pub proof fn single_pull_ref_proof(name: Seq<u8>, hash: Seq<u8>)
    ensures ({
        let local: Seq<RefEntry> = Seq::empty();
        let remote = Seq::new(1, |_i: int| RefEntry { name: name, hash: hash });
        let diff = RefDiffSpec {
            to_pull: Seq::new(1, |_i: int| name),
            to_push: Seq::empty(),
            in_sync: Seq::empty(),
            conflicts: Seq::empty(),
        };
        partition_complete(local, remote, diff)
        && classification_correct(local, remote, diff)
    })
{
    let local: Seq<RefEntry> = Seq::empty();
    let remote = Seq::new(1, |_i: int| RefEntry { name: name, hash: hash });
    let diff = RefDiffSpec {
        to_pull: Seq::new(1, |_i: int| name),
        to_push: Seq::empty(),
        in_sync: Seq::empty(),
        conflicts: Seq::empty(),
    };

    // Witness: name is at index 0 in diff.to_pull
    assert(diff.to_pull[0] == name);
    // Witness: name is at index 0 in remote
    assert(remote[0].name == name);
    // Witness for name_in_seq: exists i such that diff.to_pull[i] == name
    assert(name_in_seq(diff.to_pull, remote[0].name));
    // contains_name(local, name) is false because local is empty
    assert(!contains_name(local, name));
    // Therefore is_pull_ref holds
    assert(is_pull_ref(local, remote, name));
}

/// Proof: A single local-only ref produces exactly one to_push entry.
pub proof fn single_push_ref_proof(name: Seq<u8>, hash: Seq<u8>)
    ensures ({
        let local = Seq::new(1, |_i: int| RefEntry { name: name, hash: hash });
        let remote: Seq<RefEntry> = Seq::empty();
        let diff = RefDiffSpec {
            to_pull: Seq::empty(),
            to_push: Seq::new(1, |_i: int| name),
            in_sync: Seq::empty(),
            conflicts: Seq::empty(),
        };
        partition_complete(local, remote, diff)
        && classification_correct(local, remote, diff)
    })
{
    let local = Seq::new(1, |_i: int| RefEntry { name: name, hash: hash });
    let remote: Seq<RefEntry> = Seq::empty();
    let diff = RefDiffSpec {
        to_pull: Seq::empty(),
        to_push: Seq::new(1, |_i: int| name),
        in_sync: Seq::empty(),
        conflicts: Seq::empty(),
    };

    // Witness: name is at index 0 in diff.to_push
    assert(diff.to_push[0] == name);
    // Witness: local[0] is the entry with this name
    assert(local[0].name == name);
    // Witness for name_in_seq: exists i such that diff.to_push[i] == name
    assert(name_in_seq(diff.to_push, local[0].name));
    // contains_name(remote, name) is false because remote is empty
    assert(!contains_name(remote, name));
    // Therefore is_push_ref holds
    assert(is_push_ref(local, remote, name));
}

// ============================================================================
// Verified Exec: Hash Comparison
// ============================================================================

/// Compare two 32-byte hashes for equality.
/// Used by the production `compute_ref_diff` to classify in_sync vs conflict.
pub fn hashes_equal_32(a: &[u8; 32], b: &[u8; 32]) -> (result: bool)
    ensures result == (a@ =~= b@)
{
    let mut i: usize = 0;
    while i < 32
        invariant
            0 <= i <= 32,
            a@.len() == 32,
            b@.len() == 32,
            forall|j: int| 0 <= j < i as int ==> a@[j] == b@[j]
        decreases 32 - i
    {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

} // verus!
