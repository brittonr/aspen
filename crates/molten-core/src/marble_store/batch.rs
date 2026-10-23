use std::collections::BTreeMap;

use super::*;

/// One block a caller asks the store to stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StageRequest<'a> {
    pub digest: Digest,
    pub payload: &'a [u8],
}

/// One new object a batch writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlannedWrite<'a> {
    pub object_id: u64,
    pub digest: Digest,
    pub payload: &'a [u8],
}

/// One requested digest the index already maps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexedBlock<'a> {
    pub object_id: u64,
    pub digest: Digest,
    /// The payload the caller asked to stage, to compare with the stored one.
    pub payload: &'a [u8],
}

/// The plan of one staging batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockBatchPlan<'a> {
    /// New objects, one per new digest, in digest order.
    pub writes: Vec<PlannedWrite<'a>>,
    /// Requested digests already indexed: the shell verifies their payloads.
    pub indexed: Vec<IndexedBlock<'a>>,
    /// The identifiers the writes use; `None` when nothing is new.
    pub reservation: Option<ObjectIdReservation>,
}

/// Plan one staging batch.
///
/// Identical digests collapse to one logical object, both inside the request
/// and against the index. Two payloads under one digest are a collision.
/// When the block range cannot hold every new object the whole batch fails
/// closed: nothing is reserved and nothing is written.
// r[impl aspen.marble_store.identity]
pub fn plan_block_batch<'a>(
    requests: &[StageRequest<'a>],
    indexed: impl Fn(&Digest) -> Option<u64>,
    allocator: &ObjectIdAllocator,
) -> Result<BlockBatchPlan<'a>, MarbleStoreIssue> {
    let mut unique = BTreeMap::<Digest, &'a [u8]>::new();
    for request in requests {
        if let Some(previous) = unique.insert(request.digest, request.payload)
            && previous != request.payload
        {
            return Err(MarbleStoreIssue::DigestCollision);
        }
    }
    let mut fresh = Vec::with_capacity(unique.len());
    let mut present = Vec::with_capacity(unique.len());
    for (digest, payload) in unique {
        match indexed(&digest) {
            Some(object_id) => present.push(IndexedBlock {
                object_id,
                digest,
                payload,
            }),
            None => fresh.push((digest, payload)),
        }
    }
    if fresh.is_empty() {
        return Ok(BlockBatchPlan {
            writes: Vec::new(),
            indexed: present,
            reservation: None,
        });
    }
    let count = u32::try_from(fresh.len()).map_err(|_| MarbleStoreIssue::ObjectIdExhausted)?;
    let reservation = allocator.reserve(count)?;
    let writes = reservation
        .ids()
        .zip(fresh)
        .map(|(object_id, (digest, payload))| PlannedWrite {
            object_id,
            digest,
            payload,
        })
        .collect();
    Ok(BlockBatchPlan {
        writes,
        indexed: present,
        reservation: Some(reservation),
    })
}

/// The objects of one batch while marble writes it.
///
/// Marble's `write_batch` is crash-atomic but not runtime-atomic: while it
/// runs, its location table may expose part of the batch. Until it returns,
/// the batch is served from these owned payloads, and its mappings enter the
/// index only after a committed return.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InFlightBatch {
    objects: BTreeMap<Digest, (u64, Vec<u8>)>,
}

impl InFlightBatch {
    /// Copy the new objects of a plan into the in-flight cache.
    pub fn begin(plan: &BlockBatchPlan<'_>) -> Self {
        Self {
            objects: plan
                .writes
                .iter()
                .map(|write| (write.digest, (write.object_id, write.payload.to_vec())))
                .collect(),
        }
    }

    pub fn payload(&self, digest: &Digest) -> Option<&[u8]> {
        self.objects.get(digest).map(|(_, payload)| payload.as_slice())
    }
}

/// Where a read of one digest is served from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution<'a> {
    /// The digest belongs to the batch marble is writing.
    InFlight(&'a [u8]),
    /// The index maps the digest to a committed object.
    Indexed(u64),
    /// The store holds no block for the digest.
    Absent,
}

/// Resolve a digest: an in-flight batch takes precedence over the index.
// r[impl aspen.marble_store.identity]
// r[impl aspen.marble_store.art_index]
pub fn resolve<'a>(digest: &Digest, in_flight: Option<&'a InFlightBatch>, indexed: Option<u64>) -> Resolution<'a> {
    if let Some(payload) = in_flight.and_then(|batch| batch.payload(digest)) {
        return Resolution::InFlight(payload);
    }
    indexed.map_or(Resolution::Absent, Resolution::Indexed)
}

/// What the shell observed when marble's `write_batch` returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteBatchObservation {
    Committed,
    /// An error: marble may have installed part of the batch at runtime.
    Failed,
}

/// What to do with an in-flight batch once `write_batch` returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchSettlement {
    /// Install these digest mappings into the index.
    Install(Vec<(Digest, u64)>),
    /// Install nothing and poison the instance until it is reopened: only a
    /// reopen's recovery shows which batches marble atomically recovered.
    Poison,
}

/// Settle an in-flight batch after `write_batch` returns.
// r[impl aspen.marble_store.identity]
pub fn settle_batch(batch: InFlightBatch, observation: WriteBatchObservation) -> BatchSettlement {
    match observation {
        WriteBatchObservation::Committed => BatchSettlement::Install(
            batch.objects.into_iter().map(|(digest, (object_id, _))| (digest, object_id)).collect(),
        ),
        WriteBatchObservation::Failed => BatchSettlement::Poison,
    }
}
