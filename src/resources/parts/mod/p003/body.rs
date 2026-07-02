
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grant_consumption_throttle_and_revocation_are_receipted() {
        let grant_value = sample_grant(KIND_EFFECT_CALLS, 2, None).expect("grant");
        let grant = parse_resource_grant(&grant_value).expect("parse grant");
        let first = consume_resource(&ConsumeInput {
            grant_value: &grant_value,
            prior_consumptions: &[],
            amount: 1,
            logical_time: 0,
            sequence: 0,
            is_revoked: false,
        })
        .expect("first consume");
        assert_eq!(first.decision, "pass");
        let consumption = parse_consumption(&resource_consumption_value(&grant, 1, 0).expect("consumption"))
            .expect("parse consumption");
        let prior_consumptions = [consumption];
        let second = consume_resource(&ConsumeInput {
            grant_value: &grant_value,
            prior_consumptions: &prior_consumptions,
            amount: 2,
            logical_time: 0,
            sequence: 1,
            is_revoked: false,
        })
        .expect("over consume");
        assert_eq!(second.decision, "throttle");
        let revoked = consume_resource(&ConsumeInput {
            grant_value: &grant_value,
            prior_consumptions: &[],
            amount: 1,
            logical_time: 0,
            sequence: 2,
            is_revoked: true,
        })
        .expect("revoked consume");
        assert_eq!(revoked.decision, "deny");
    }

    #[test]
    fn mailbox_overflow_is_deterministic_and_not_silent() {
        let first = ref_for("message-1");
        let second = ref_for("message-2");
        let accepted = apply_mailbox_backpressure(&[], &first, 1).expect("accepted");
        assert!(accepted.accepted);
        let denied = apply_mailbox_backpressure(&accepted.queue, &second, 1).expect("overflow");
        assert!(!denied.accepted);
        assert_eq!(denied.overflow, Some(second));
        assert!(crate::preserves_rail::to_text(&denied.receipt_value).expect("receipt").contains("mailbox-full"));
    }

    #[test]
    fn turn_assertion_adapter_and_job_budgets_are_enforced() {
        let turn_grant = sample_grant(KIND_TURNS, 1, None).expect("turn grant");
        assert_eq!(
            consume_resource(&ConsumeInput {
                grant_value: &turn_grant,
                prior_consumptions: &[],
                amount: 1,
                logical_time: 0,
                sequence: 0,
                is_revoked: false,
            })
            .expect("turn")
            .decision,
            "pass"
        );
        assert_eq!(
            consume_resource(&ConsumeInput {
                grant_value: &turn_grant,
                prior_consumptions: &[],
                amount: 2,
                logical_time: 0,
                sequence: 1,
                is_revoked: false,
            })
            .expect("turn over")
            .decision,
            "throttle"
        );
        assert_eq!(enforce_assertion_bound(1, 1, &ref_for("assertion")).expect("assertion").decision, "deny");
        assert_eq!(adapter_budget_decision(KIND_CPU_FUEL, 10, 8, "wasmtime-fuel").expect("wasm").decision, "deny");
        assert_eq!(
            adapter_budget_decision(KIND_CPU_FUEL, 4, 8, "steel-native-budget").expect("steel").decision,
            "pass"
        );
        assert_eq!(
            adapter_budget_decision(KIND_BLOB_BYTES, 9, 8, "blob-storage-network").expect("blob").decision,
            "deny"
        );
        assert_eq!(plan_job_stages(&[("a", 1), ("b", 2)], 2).expect("plan"), vec!["place:a:1", "defer:b:2"]);
    }

    #[test]
    fn deterministic_scheduler_is_os_timing_independent() {
        let tasks = vec![
            SchedulerTask {
                actor: "a".to_string(),
                priority: 0,
                sequence: 1,
                budget_class: "normal".to_string(),
            },
            SchedulerTask {
                actor: "b".to_string(),
                priority: 0,
                sequence: 2,
                budget_class: "normal".to_string(),
            },
        ];
        let first = deterministic_schedule(&tasks, 1).expect("schedule");
        let second = deterministic_schedule(&tasks, 1).expect("schedule");
        assert_eq!(first, second);
        assert!(crate::preserves_rail::to_text(&first).expect("schedule text").contains("os-timing-independent"));
    }

    #[test]
    fn expired_grants_deny_future_work_and_receipts_replay() {
        let grant_value = sample_grant(KIND_NETWORK_MESSAGES, 1, Some(5)).expect("grant");
        let before = consume_resource(&ConsumeInput {
            grant_value: &grant_value,
            prior_consumptions: &[],
            amount: 1,
            logical_time: 4,
            sequence: 0,
            is_revoked: false,
        })
        .expect("before expiry");
        let after = consume_resource(&ConsumeInput {
            grant_value: &grant_value,
            prior_consumptions: &[],
            amount: 1,
            logical_time: 5,
            sequence: 1,
            is_revoked: false,
        })
        .expect("after expiry");
        assert_eq!(before.decision, "pass");
        assert_eq!(after.decision, "deny");
        let replay = consume_resource(&ConsumeInput {
            grant_value: &grant_value,
            prior_consumptions: &[],
            amount: 1,
            logical_time: 5,
            sequence: 1,
            is_revoked: false,
        })
        .expect("replay");
        assert_eq!(after.receipt_value, replay.receipt_value);
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_budget_monotonicity_queue_bounds_and_no_silent_drop(tc: hegel::TestCase) {
        let amount = tc.draw(hegel::generators::integers::<u64>().min_value(1).max_value(16));
        let request = tc.draw(hegel::generators::integers::<u64>().min_value(1).max_value(20));
        let grant_value = sample_grant(KIND_TRACE_BYTES, amount, None).expect("grant");
        let decision = consume_resource(&ConsumeInput {
            grant_value: &grant_value,
            prior_consumptions: &[],
            amount: request,
            logical_time: 0,
            sequence: 0,
            is_revoked: false,
        })
        .expect("consume");
        if request <= amount {
            assert_eq!(decision.decision, "pass");
        } else {
            assert_eq!(decision.decision, "throttle");
            assert_eq!(decision.consumed, 0);
        }
        let max_slots = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(4));
        let max_slots_usize = usize::try_from(max_slots).expect("bounded max slots");
        let queue = (0..max_slots_usize).map(|index| ref_for(&format!("queued-{index}"))).collect::<Vec<_>>();
        let mailbox = apply_mailbox_backpressure(&queue, &ref_for("new-message"), max_slots).expect("mailbox");
        assert_eq!(mailbox.queue.len(), max_slots_usize);
        assert!(!mailbox.accepted);
        assert!(mailbox.overflow.is_some());
    }

    fn sample_grant(kind: &str, amount: u64, expires_at: Option<u64>) -> Result<IoValue> {
        resource_grant_value(&ResourceGrantInput {
            subject_ref: ref_for("subject"),
            scope: "scope".to_string(),
            kind: kind.to_string(),
            amount,
            rate: None,
            window: None,
            not_before: None,
            expires_at,
            parent_ref: None,
            revocation_refs: Vec::new(),
            policy_refs: vec![ref_for("policy")],
            evidence_refs: vec![ref_for("evidence")],
        })
    }

    fn ref_for(label: &str) -> String {
        canonical_hash(&record("resource-test-ref", vec![string(label)])).expect("test ref")
    }
}
