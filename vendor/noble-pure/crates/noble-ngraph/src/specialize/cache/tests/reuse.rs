use super::*;

#[test]
fn identical_request_hits_dynamic_inputs_and_rebinding_keeps_old_owner() {
    let original = graph(&TestIds, false, 7, &[11, 12], false, false, false);
    let mut session = Session::default();
    let first =
        run(&mut session, &TestIds, &original, &[Some(5), None]).expect("first specialization");
    assert_eq!(first.status, Status::MissStored);
    let old = first.residual.clone();
    for dynamic in [0, 1, -2, i64::MIN] {
        assert_eq!(
            evaluate(&old.checked, &[dynamic]),
            evaluate(&original, &[5, dynamic])
        );
        assert_eq!(
            evaluate(&first.readmitted, &[dynamic]),
            evaluate(&original, &[5, dynamic])
        );
    }
    let warm = run(&mut session, &TestIds, &original, &[Some(5), None]).expect("identical request");
    assert_eq!(warm.status, Status::Hit);
    assert_eq!(warm.source, first.source);
    assert_eq!(warm.specialization_key, first.specialization_key);
    assert_eq!(warm.program_key, first.program_key);
    assert_eq!(
        evaluate(&warm.readmitted, &[3]),
        evaluate(&original, &[5, 3])
    );

    let rebound = graph(&TestIds, true, 7, &[11, 12], false, false, false);
    let changed = run(&mut session, &TestIds, &rebound, &[Some(5), None])
        .expect("rebound source specializes independently");
    assert_eq!(changed.status, Status::MissStored);
    assert_ne!(changed.residual.graph().nodes, old.graph().nodes);
    assert_eq!(evaluate(&old.checked, &[3]), evaluate(&original, &[5, 3]));
    assert_eq!(
        evaluate(&changed.residual.checked, &[3]),
        evaluate(&rebound, &[5, 3])
    );
    assert_eq!(
        evaluate(&changed.readmitted, &[3]),
        evaluate(&rebound, &[5, 3])
    );
}
