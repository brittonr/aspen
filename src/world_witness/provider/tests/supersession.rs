fn bind(digest: u8, predecessor: Option<[u8; 32]>) -> super::super::ordinal::Candidate {
    super::super::ordinal::Candidate {
        digest: [digest; 32],
        predecessor,
    }
}

fn committed_first(
    provider: &mut super::super::Index<super::Fake, crate::world_witness::LivePorts>,
) -> release_channel_index::model::Claim {
    let first = super::claim(1, None);
    let reserved = provider.reserve(1, first.clone()).expect("reserve first");
    provider.commit(&reserved).expect("commit first");
    first
}

/// Abandons one externally reserved successor whose consumer CAS did not commit.
fn abandon(
    provider: &mut super::super::Index<super::Fake, crate::world_witness::LivePorts>,
    predecessor: [u8; 32],
) -> (super::super::Reservation, release_channel_index::model::Snapshot) {
    let revision = provider.query(0).expect("query").snapshot.revision;
    let abandoned = release_channel_index::model::Claim {
        generation: 2,
        digest: [0xAB; 32],
        predecessor: Some(predecessor),
    };
    let reserved = provider.reserve(revision, abandoned).expect("reserve abandoned");
    let orphan = provider.orphan(&reserved).expect("orphan proven-uncommitted CAS").snapshot;
    (reserved, orphan)
}

// r[verify molten.world_witness.verification]
#[test]
fn resolved_orphan_high_water_is_the_only_admitted_external_gap() {
    let mut provider = super::fixture();
    let first = committed_first(&mut provider);
    let successor = bind(2, Some(first.digest));
    let (abandoned, unresolved) = abandon(&mut provider, first.digest);
    assert!(super::super::ordinal::plan(&unresolved, &successor).is_err());
    let resolved = provider.resolve_orphan(&abandoned).expect("explicit resolution").snapshot;
    // The head successor keeps Molten generation two inside its claim digest; the
    // provider rejects reusing external ordinal two after the abandoned high-water.
    let plain = release_channel_index::model::Claim {
        generation: 2,
        digest: successor.digest,
        predecessor: successor.predecessor,
    };
    assert!(provider.reserve(resolved.revision, plain).is_err());
    let planned = super::super::ordinal::plan(&resolved, &successor).expect("authenticated gap");
    assert_eq!(planned.generation, 3);
    assert_eq!(planned.predecessor, Some(first.digest));
    assert!(super::super::ordinal::plan(&resolved, &bind(0xAB, Some(first.digest))).is_err());
    let reserved = provider.reserve(resolved.revision, planned.clone()).expect("reserve successor");
    provider.commit(&reserved).expect("commit successor");
    provider.observe_current(&planned, 0).expect("superseding successor is current");
    assert!(provider.observe_current(&first, 0).is_err());
}

// r[verify molten.world_witness.verification]
#[test]
fn lost_reserve_across_gap_recovers_the_signed_external_position() {
    let mut provider = super::fixture();
    let first = committed_first(&mut provider);
    let (abandoned, _) = abandon(&mut provider, first.digest);
    let resolved = provider.resolve_orphan(&abandoned).expect("explicit resolution").snapshot;
    let successor = bind(2, Some(first.digest));
    let planned = super::super::ordinal::plan(&resolved, &successor).expect("authenticated gap");
    provider.transport.lose_reserve = true;
    let error = provider.reserve(resolved.revision, planned.clone()).expect_err("lost response");
    assert_eq!(error.kind, super::super::ErrorKind::Uncertain);
    let recovered = provider.recover_reservation(&successor, 0).expect("query").expect("pending");
    assert_eq!(recovered.claim, planned);
    let retried = provider.query(0).expect("query").snapshot;
    assert_eq!(super::super::ordinal::plan(&retried, &successor).expect("exact pending"), planned);
}

// r[verify molten.world_witness.verification]
#[test]
fn rolled_back_predecessor_and_forged_gaps_cannot_plan_a_successor() {
    let empty = release_channel_index::model::Snapshot {
        revision: 1,
        highest: None,
        committed: None,
        pending: None,
    };
    let initial = super::super::ordinal::plan(&empty, &bind(1, None)).expect("explicit enrollment");
    assert_eq!(initial, super::claim(1, None));
    assert!(super::super::ordinal::plan(&empty, &bind(1, Some([1; 32]))).is_err());
    let mut provider = super::fixture();
    let first = committed_first(&mut provider);
    let current = provider.query(0).expect("query").snapshot;
    let second = super::super::ordinal::plan(&current, &bind(2, Some(first.digest))).expect("next");
    let reserved = provider.reserve(current.revision, second.clone()).expect("reserve");
    provider.commit(&reserved).expect("commit");
    let advanced = provider.query(0).expect("query").snapshot;
    // A whole-consumer rollback still names the first claim as its predecessor.
    assert!(super::super::ordinal::plan(&advanced, &bind(3, Some(first.digest))).is_err());
    let mut forged = advanced.clone();
    forged.highest = Some(release_channel_index::model::Claim {
        generation: 9,
        digest: [9; 32],
        predecessor: Some(first.digest),
    });
    assert!(super::super::ordinal::plan(&forged, &bind(3, Some(second.digest))).is_err());
}
