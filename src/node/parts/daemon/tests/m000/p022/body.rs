
    fn write_active_service_lock(root: &Path, service_suffix: &str) {
        let state_root = crate::node_state::NodeStateRoot::open(root).expect("open node state root");
        let startup = current_startup_receipt(&state_root).expect("startup");
        let identity = crate::node_identity::parse_identity(
            &read_preserves(
                &state_root,
                &crate::node_state::NodeStatePath::parse(IDENTITY_FILE).expect("identity path"),
            )
            .expect("identity"),
        )
        .expect("parse identity");
        let service_run_ref = local_ref("node-control-service-run", service_suffix).expect("service run ref");
        let lock_value = service_lock_value(&ServiceLockValueInput {
            startup_receipt_ref: &startup.receipt_ref,
            node_id: &identity.node_id,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            max_ticks: 1,
            max_requests_per_tick: 1,
            service_run_ref: &service_run_ref,
        })
        .expect("service lock");
        write_preserves(
            &state_root,
            &crate::node_state::NodeStatePath::parse(CONTROL_SERVICE_LOCK_FILE).expect("service lock path"),
            &lock_value,
        )
        .expect("write service lock");
    }

    fn recovering_policy(policy_refs: &[String]) -> IoValue {
        control_supervisor_policy_value(&ControlSupervisorPolicyInput {
            max_restarts: 1,
            restart_window_ticks: 1,
            heartbeat_timeout_ticks: 1,
            shutdown_drain_ticks: 1,
            stale_lock_recovery: true,
            policy_refs,
            evidence_refs: &[],
        })
        .expect("recover policy")
    }

    fn bounded_shutdown_policy(policy_refs: &[String]) -> IoValue {
        control_supervisor_policy_value(&ControlSupervisorPolicyInput {
            max_restarts: 0,
            restart_window_ticks: 1,
            heartbeat_timeout_ticks: 1,
            shutdown_drain_ticks: 0,
            stale_lock_recovery: false,
            policy_refs,
            evidence_refs: &[],
        })
        .expect("tight policy")
    }
