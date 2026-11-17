use super::*;

struct CollidingIds;

struct ConstantHasher;

impl crate::identity::IdentityHasher for ConstantHasher {
    fn update(&mut self, _bytes: &[u8]) {}

    fn finish(self: alloc::boxed::Box<Self>) -> crate::ir::Digest {
        [0; 32]
    }
}

impl IdentityScheme for CollidingIds {
    fn name(&self) -> &'static str {
        "constant-test"
    }

    fn start(&self) -> alloc::boxed::Box<dyn crate::identity::IdentityHasher + '_> {
        alloc::boxed::Box::new(ConstantHasher)
    }
}

#[test]
fn colliding_lookup_hashes_require_full_manifest_equality() {
    let original = graph(&CollidingIds, false, 7, &[11, 12], false, false, false);
    let changed = graph(&CollidingIds, true, 7, &[11, 12], false, false, false);
    let mut session = Session::default();
    let first = run(&mut session, &CollidingIds, &original, &[Some(5), None]).unwrap();
    let second = run(&mut session, &CollidingIds, &changed, &[Some(5), None]).unwrap();
    assert_eq!(first.status, Status::MissStored);
    assert_eq!(second.status, Status::MissStored);
    assert_eq!(first.specialization_key, second.specialization_key);
    assert_eq!(first.program_key, second.program_key);
    assert_eq!(
        evaluate(&first.readmitted, &[3]),
        evaluate(&original, &[5, 3])
    );
    assert_eq!(
        evaluate(&second.readmitted, &[3]),
        evaluate(&changed, &[5, 3])
    );
    assert_ne!(
        evaluate(&first.readmitted, &[3]),
        evaluate(&second.readmitted, &[3])
    );
}

#[test]
fn stale_truncated_and_corrupt_entries_are_rebuilt() {
    for corruption in 0..3 {
        let original = graph(&TestIds, false, 7, &[11, 12], false, false, false);
        let mut session = Session::default();
        run(&mut session, &TestIds, &original, &[Some(5), None]).unwrap();
        match corruption {
            0 => session.entries[0].subject_manifest[0] ^= 1,
            1 => session.entries[0].graph_text.truncate(1),
            _ => session.entries[0].source[0] ^= 1,
        }
        let rebuilt = run(&mut session, &TestIds, &original, &[Some(5), None]).unwrap();
        assert_eq!(rebuilt.status, Status::CorruptRebuilt);
        assert_eq!(
            evaluate(&rebuilt.readmitted, &[3]),
            evaluate(&original, &[5, 3])
        );
    }
}

#[test]
fn unrelated_well_typed_residual_cannot_substitute_cached_behavior() {
    let original = graph(&TestIds, false, 7, &[11, 12], false, false, false);
    let unrelated = graph(&TestIds, true, 7, &[11, 12], false, false, false);
    let mut session = Session::default();
    run(&mut session, &TestIds, &original, &[Some(5), None]).unwrap();
    let budgets = crate::check::Budgets::default();
    let residual = crate::specialize::specialize(
        &TestIds,
        &unrelated,
        &crate::specialize::Request {
            static_inputs: &[Some(5), None],
            budgets: &budgets,
        },
    )
    .expect("unrelated residual is well-typed");
    let graph_text = crate::format::graph::print(residual.checked.graph());
    let source = crate::specialize::emit_source(&residual.checked).expect("unrelated source");
    session.entries[0].graph_text = graph_text.into_bytes();
    session.entries[0].source = source.into_bytes();

    let rebuilt = run(&mut session, &TestIds, &original, &[Some(5), None]).unwrap();
    assert_eq!(rebuilt.status, Status::CorruptRebuilt);
    assert_eq!(
        evaluate(&rebuilt.readmitted, &[3]),
        evaluate(&original, &[5, 3])
    );
    assert_ne!(
        evaluate(&rebuilt.readmitted, &[3]),
        evaluate(&unrelated, &[5, 3])
    );
}

#[test]
fn swapped_artifact_bytes_do_not_cross_substitute_requests() {
    let original = graph(&TestIds, false, 7, &[11, 12], false, false, false);
    let mut session = Session::default();
    run(&mut session, &TestIds, &original, &[Some(5), None]).unwrap();
    run(&mut session, &TestIds, &original, &[Some(6), None]).unwrap();
    let (first, second) = session.entries.split_at_mut(1);
    core::mem::swap(&mut first[0].graph_text, &mut second[0].graph_text);
    core::mem::swap(&mut first[0].source, &mut second[0].source);

    let rebuilt = run(&mut session, &TestIds, &original, &[Some(5), None]).unwrap();
    assert_eq!(rebuilt.status, Status::CorruptRebuilt);
    assert_eq!(
        evaluate(&rebuilt.readmitted, &[3]),
        evaluate(&original, &[5, 3])
    );
}

#[test]
fn wrong_evidence_and_provenance_force_rebuilds() {
    for wrong_evidence in [false, true] {
        let original = graph(&TestIds, false, 7, &[11, 12], false, false, false);
        let mut session = Session::default();
        run(&mut session, &TestIds, &original, &[Some(5), None]).unwrap();
        if wrong_evidence {
            session.entries[0].report.residual_graph[0] ^= 1;
        } else {
            session.entries[0].provenance[0] ^= 1;
        }
        let rebuilt = run(&mut session, &TestIds, &original, &[Some(5), None]).unwrap();
        assert_eq!(rebuilt.status, Status::CorruptRebuilt);
        assert_eq!(
            evaluate(&rebuilt.readmitted, &[3]),
            evaluate(&original, &[5, 3])
        );
    }
}
