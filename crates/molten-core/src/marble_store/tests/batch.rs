use super::*;

fn request<'a>(label: &str, payload: &'a [u8]) -> StageRequest<'a> {
    StageRequest {
        digest: digest(label),
        payload,
    }
}

// r[verify aspen.marble_store.identity]
#[test]
fn identical_digests_collapse_to_one_object_and_indexed_digests_are_not_rewritten() {
    let allocator = ObjectIdAllocator::seeded(layout(16), 0).expect("allocator");
    let indexed = digest("indexed");
    let plan = plan_block_batch(
        &[
            request("a", b"a"),
            request("a", b"a"),
            request("indexed", b"i"),
            request("b", b"b"),
        ],
        |candidate| (*candidate == indexed).then_some(1_030),
        &allocator,
    )
    .expect("plan");
    assert_eq!(plan.writes.len(), 2, "one write per new digest");
    assert_eq!(plan.indexed.len(), 1);
    assert_eq!(plan.indexed[0].object_id, 1_030);
    let ids = plan.writes.iter().map(|write| write.object_id).collect::<Vec<_>>();
    assert_eq!(ids, vec![1_024, 1_025]);
    assert_eq!(
        plan_block_batch(&[request("a", b"a"), request("a", b"z")], |_| None, &allocator),
        Err(MarbleStoreIssue::DigestCollision)
    );
}

// r[verify aspen.marble_store.verification]
#[test]
fn object_id_exhaustion_fails_the_whole_batch_closed() {
    let mut allocator = ObjectIdAllocator::seeded(layout(3), 0).expect("allocator");
    let requests = [
        request("a", b"a"),
        request("b", b"b"),
        request("c", b"c"),
        request("d", b"d"),
    ];
    assert_eq!(plan_block_batch(&requests, |_| None, &allocator), Err(MarbleStoreIssue::ObjectIdExhausted));
    assert_eq!(allocator.remaining(), 3, "a refused batch reserves nothing");

    let plan = plan_block_batch(&requests[..3], |_| None, &allocator).expect("fits");
    allocator.commit(plan.reservation.as_ref().expect("reservation"));
    assert_eq!(allocator.remaining(), 0);
    assert_eq!(allocator.reserve(1), Err(MarbleStoreIssue::ObjectIdExhausted));
    assert_eq!(allocator.reserve(0).map(|reservation| reservation.count), Ok(0));
}

#[test]
fn the_allocator_reseeds_past_every_identifier_the_store_holds() {
    let reseeded = ObjectIdAllocator::seeded(layout(16), 1_030).expect("allocator");
    assert_eq!(reseeded.reserve(1).map(|reservation| reservation.first), Ok(1_030));
    let fresh = ObjectIdAllocator::seeded(layout(16), 1).expect("only the catalog");
    assert_eq!(fresh.reserve(1).map(|reservation| reservation.first), Ok(1_024));
    let full = ObjectIdAllocator::seeded(layout(16), 1_040).expect("full range");
    assert_eq!(full.remaining(), 0);
    assert_eq!(ObjectIdAllocator::seeded(layout(16), 500), Err(MarbleStoreIssue::ForeignObjectId(499)));
    assert_eq!(ObjectIdAllocator::seeded(layout(16), 1_041), Err(MarbleStoreIssue::ForeignObjectId(1_040)));
}

// r[verify aspen.marble_store.identity]
#[test]
fn an_in_flight_batch_is_served_from_its_cache_and_installs_only_on_commit() {
    let allocator = ObjectIdAllocator::seeded(layout(16), 0).expect("allocator");
    let plan = plan_block_batch(&[request("new", b"new")], |_| None, &allocator).expect("plan");
    let batch = InFlightBatch::begin(&plan);
    assert_eq!(resolve(&digest("new"), Some(&batch), None), Resolution::InFlight(b"new".as_slice()));
    assert_eq!(resolve(&digest("new"), Some(&batch), Some(9)), Resolution::InFlight(b"new".as_slice()));
    assert_eq!(resolve(&digest("old"), Some(&batch), Some(9)), Resolution::Indexed(9));
    assert_eq!(resolve(&digest("absent"), Some(&batch), None), Resolution::Absent);
    assert_eq!(resolve(&digest("new"), None, None), Resolution::Absent, "nothing is indexed before commit");

    assert_eq!(
        settle_batch(batch.clone(), WriteBatchObservation::Committed),
        BatchSettlement::Install(vec![(digest("new"), 1_024)])
    );
    assert_eq!(settle_batch(batch, WriteBatchObservation::Failed), BatchSettlement::Poison);
}
