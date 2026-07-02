
    #[hegel::test(test_cases = 16)]
    fn hegel_promise_pipeline_ordering_bounds_and_terminal_cleanup(tc: TestCase) {
        let queue_len = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(4));
        let queue = (0..queue_len)
            .map(|index| {
                crate::runtime::RuntimePromisePipelineEntry::new(
                    index + 1,
                    vat_test_ref(&format!("target-{index}")),
                    "call",
                )
            })
            .collect::<Vec<_>>();
        let pending = crate::runtime::evaluate_promise_pipeline(&crate::runtime::RuntimePromisePipelineState::new(
            crate::runtime::RuntimePromiseState::pending("promise:hegel"),
            PIPELINE_MAX_QUEUE,
            queue,
        ))
        .expect("pending pipeline");
        assert_eq!(pending.receipt.decision, crate::runtime::PredicateDecision::Pass);

        let overflow_len = tc.draw(hegel::generators::integers::<u64>().min_value(5).max_value(8));
        let overflow_queue = (0..overflow_len)
            .map(|index| {
                crate::runtime::RuntimePromisePipelineEntry::new(
                    index + 1,
                    vat_test_ref(&format!("overflow-{index}")),
                    "call",
                )
            })
            .collect::<Vec<_>>();
        let overflow = crate::runtime::evaluate_promise_pipeline(&crate::runtime::RuntimePromisePipelineState::new(
            crate::runtime::RuntimePromiseState::pending("promise:overflow"),
            PIPELINE_MAX_QUEUE,
            overflow_queue,
        ))
        .expect("overflow pipeline");
        assert_eq!(overflow.receipt.decision, crate::runtime::PredicateDecision::Deny);

        let terminal = crate::runtime::evaluate_promise_pipeline(&crate::runtime::RuntimePromisePipelineState::new(
            crate::runtime::RuntimePromiseState::broken("promise:terminal", "causal failure", Vec::new()),
            PIPELINE_MAX_QUEUE,
            vec![crate::runtime::RuntimePromisePipelineEntry::new(
                1,
                vat_test_ref("stale-terminal"),
                "late-call",
            )],
        ))
        .expect("terminal pipeline");
        assert_eq!(terminal.receipt.decision, crate::runtime::PredicateDecision::Deny);
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_actormap_commit_and_rollback_invariants(tc: TestCase) {
        let spawn_count = tc.draw(hegel::generators::integers::<u64>().min_value(1).max_value(4));
        let before = sorted_refs(vec![vat_test_ref("root"), vat_test_ref("helper")]);
        let spawned =
            sorted_refs((0..spawn_count).map(|index| vat_test_ref(&format!("spawned-{index}"))).collect::<Vec<_>>());
        let after = sorted_refs(before.iter().cloned().chain(spawned.iter().cloned()).collect());
        let committed =
            crate::runtime::evaluate_actormap_transaction(&crate::runtime::RuntimeActormapTransactionState {
                outcome: crate::runtime::RuntimeActormapTransactionOutcome::Committed,
                before_object_refs: before.clone(),
                after_object_refs: after.clone(),
                spawned_object_refs: spawned.clone(),
                removed_object_refs: Vec::new(),
                visible_object_refs: after,
                used_object_refs: vec![before[0].clone()],
            })
            .expect("commit");
        assert_eq!(committed.receipt.decision, crate::runtime::PredicateDecision::Pass);

        let rollback =
            crate::runtime::evaluate_actormap_transaction(&crate::runtime::RuntimeActormapTransactionState {
                outcome: crate::runtime::RuntimeActormapTransactionOutcome::RolledBack,
                before_object_refs: before.clone(),
                after_object_refs: before.clone(),
                spawned_object_refs: spawned.clone(),
                removed_object_refs: Vec::new(),
                visible_object_refs: before.clone(),
                used_object_refs: Vec::new(),
            })
            .expect("rollback");
        assert_eq!(rollback.receipt.decision, crate::runtime::PredicateDecision::Pass);

        let leaked_spawn =
            crate::runtime::evaluate_actormap_transaction(&crate::runtime::RuntimeActormapTransactionState {
                outcome: crate::runtime::RuntimeActormapTransactionOutcome::RolledBack,
                before_object_refs: before.clone(),
                after_object_refs: before,
                spawned_object_refs: spawned.clone(),
                removed_object_refs: Vec::new(),
                visible_object_refs: spawned,
                used_object_refs: Vec::new(),
            })
            .expect("leaked rollback");
        assert_eq!(leaked_spawn.receipt.decision, crate::runtime::PredicateDecision::Deny);
    }

    fn vat_test_ref(label: &str) -> String {
        canonical_hash(&string(format!("vat-test:{label}"))).expect("vat test ref")
    }
