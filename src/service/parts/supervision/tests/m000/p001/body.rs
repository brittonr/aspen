
    #[hegel::test(test_cases = 16)]
    fn hegel_cleanup_bounded_and_monitor_order_deterministic(tc: TestCase) {
        let attempt = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(3));
        let suite_value = suite_with_attempt(attempt);
        let run = run_service_supervision_suite_value(&suite_value).expect("generated supervision run");
        let replay = replay_service_supervision_report(&run.value).expect("generated replay");
        assert_eq!(replay.decision, "pass");
        let is_restart_denied = attempt >= 2;
        if is_restart_denied {
            assert_eq!(run.cleanup_receipts.len(), 1);
            assert!(run.scheduled_demands.is_empty());
        } else {
            assert!(run.cleanup_receipts.is_empty());
            assert_eq!(run.scheduled_demands.len(), 1);
        }
        let second_run = run_service_supervision_suite_value(&suite_value).expect("rerun generated supervision");
        assert_eq!(run.monitor_notifications, second_run.monitor_notifications);
    }
