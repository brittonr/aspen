use std::collections::BTreeSet;

use crate::quorum::quorum_set::QuorumSet;

const QUORUM_VOTE_WEIGHT_STEP: usize = 2;

fn has_simple_majority<'a, ID, I>(
    member_count: usize,
    ids: I,
    contains_member: impl Fn(&ID) -> bool,
) -> bool
where
    ID: 'a,
    I: Iterator<Item = &'a ID> + Clone,
{
    let quorum_vote_weight_threshold = member_count;
    let mut granted_vote_weight = 0_usize;
    for id in ids {
        if contains_member(id) {
            granted_vote_weight = granted_vote_weight.saturating_add(QUORUM_VOTE_WEIGHT_STEP);
            if granted_vote_weight > quorum_vote_weight_threshold {
                return true;
            }
        }
    }
    false
}

/// Impl a simple majority quorum set
impl<ID> QuorumSet<ID> for BTreeSet<ID>
where ID: PartialOrd + Ord + Clone + 'static
{
    type Iter = std::collections::btree_set::IntoIter<ID>;

    fn is_quorum<'a, I: Iterator<Item = &'a ID> + Clone>(&self, ids: I) -> bool {
        has_simple_majority(self.len(), ids, |id| self.contains(id))
    }

    fn ids(&self) -> Self::Iter {
        self.clone().into_iter()
    }
}

/// Impl a simple majority quorum set
impl<ID> QuorumSet<ID> for Vec<ID>
where ID: PartialOrd + Ord + Clone + 'static
{
    type Iter = std::collections::btree_set::IntoIter<ID>;

    fn is_quorum<'a, I: Iterator<Item = &'a ID> + Clone>(&self, ids: I) -> bool {
        has_simple_majority(self.len(), ids, |id| self.contains(id))
    }

    fn ids(&self) -> Self::Iter {
        BTreeSet::from_iter(self.iter().cloned()).into_iter()
    }
}

/// Impl a simple majority quorum set
impl<ID> QuorumSet<ID> for &[ID]
where ID: PartialOrd + Ord + Copy + 'static
{
    type Iter = std::collections::btree_set::IntoIter<ID>;

    fn is_quorum<'a, I: Iterator<Item = &'a ID> + Clone>(&self, ids: I) -> bool {
        has_simple_majority(self.len(), ids, |id| self.contains(id))
    }

    fn ids(&self) -> Self::Iter {
        BTreeSet::from_iter(self.iter().copied()).into_iter()
    }
}
