use super::*;

/// A half-open range `[start, end)` of marble object identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectIdRange {
    pub start: u64,
    pub end: u64,
}

impl ObjectIdRange {
    pub const fn contains(&self, object_id: u64) -> bool {
        self.start <= object_id && object_id < self.end
    }

    const fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    const fn overlaps(&self, other: &Self) -> bool {
        self.start < other.end && other.start < self.end
    }
}

/// Identifier ranges sharded by object kind.
///
/// The catalog is the first identifier of its range; blocks take identifiers
/// from their own range in increasing order. Sharding keeps the catalog and
/// the blocks apart so marble maintenance can partition them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectIdLayout {
    pub catalog: ObjectIdRange,
    pub blocks: ObjectIdRange,
}

impl ObjectIdLayout {
    /// The fixed identifier of the catalog object.
    pub const fn catalog_id(&self) -> u64 {
        self.catalog.start
    }

    /// The kind an identifier belongs to, or `None` for a foreign identifier.
    pub const fn kind_of(&self, object_id: u64) -> Option<ObjectKind> {
        if self.catalog.contains(object_id) {
            Some(ObjectKind::Catalog)
        } else if self.blocks.contains(object_id) {
            Some(ObjectKind::Block)
        } else {
            None
        }
    }
}

pub(super) fn validate_layout(layout: &ObjectIdLayout) -> Vec<MarbleStoreIssue> {
    let mut issues = Vec::with_capacity(3);
    if layout.catalog.is_empty() {
        issues.push(MarbleStoreIssue::EmptyRange(ObjectKind::Catalog));
    }
    if layout.blocks.is_empty() {
        issues.push(MarbleStoreIssue::EmptyRange(ObjectKind::Block));
    }
    if layout.catalog.overlaps(&layout.blocks) {
        issues.push(MarbleStoreIssue::OverlappingRanges);
    }
    issues
}

/// Allocates block identifiers in increasing order inside the block range.
///
/// The allocator never reuses an identifier while the instance lives. It is
/// reseeded at open from the store's high-water mark, so an identifier marble
/// still records, including a deletion, is never handed out twice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectIdAllocator {
    layout: ObjectIdLayout,
    next_block: u64,
}

/// Identifiers reserved for one batch; they become used only on commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectIdReservation {
    pub first: u64,
    pub count: u32,
}

impl ObjectIdReservation {
    /// The reserved identifiers, in order.
    pub fn ids(&self) -> impl Iterator<Item = u64> + '_ {
        (0..u64::from(self.count)).map(|offset| self.first.saturating_add(offset))
    }
}

impl ObjectIdAllocator {
    /// Seed the allocator from one past the highest identifier the store holds.
    ///
    /// A high-water mark beyond the block range is accepted only when the
    /// store holds nothing past the range: it leaves no identifier to hand out.
    pub fn seeded(layout: ObjectIdLayout, high_water: u64) -> Result<Self, MarbleStoreIssue> {
        let issues = validate_layout(&layout);
        if let Some(issue) = issues.into_iter().next() {
            return Err(issue);
        }
        let highest = high_water.checked_sub(1);
        if let Some(highest) = highest
            && layout.kind_of(highest).is_none()
        {
            return Err(MarbleStoreIssue::ForeignObjectId(highest));
        }
        let next_block = high_water.max(layout.blocks.start);
        Ok(Self { layout, next_block })
    }

    pub const fn layout(&self) -> &ObjectIdLayout {
        &self.layout
    }

    /// Reserve `count` consecutive block identifiers, or fail closed.
    // r[impl aspen.marble_store.identity]
    pub fn reserve(&self, count: u32) -> Result<ObjectIdReservation, MarbleStoreIssue> {
        let end = self.next_block.checked_add(u64::from(count)).ok_or(MarbleStoreIssue::ObjectIdExhausted)?;
        if end > self.layout.blocks.end {
            return Err(MarbleStoreIssue::ObjectIdExhausted);
        }
        Ok(ObjectIdReservation {
            first: self.next_block,
            count,
        })
    }

    /// Mark a committed reservation as used.
    pub fn commit(&mut self, reservation: &ObjectIdReservation) {
        let end = reservation.first.saturating_add(u64::from(reservation.count));
        self.next_block = self.next_block.max(end);
    }

    /// Identifiers still available in the block range.
    pub const fn remaining(&self) -> u64 {
        self.layout.blocks.end.saturating_sub(self.next_block)
    }
}
