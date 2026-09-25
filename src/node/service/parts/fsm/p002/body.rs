
#[cfg(test)]
mod tests {
    use super::*;

    fn local_ref(label: &str) -> String {
        crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
    }

    fn refs(label: &str) -> Vec<String> {
        vec![local_ref(label)]
    }

    fn state(name: &str, startup: Option<String>, lock: Option<String>) -> ServiceFsmState {
        let state_ref = service_state_ref(name, startup.as_deref(), lock.as_deref(), 0, 0).expect("state ref");
        ServiceFsmState {
            state_name: name.to_string(),
            state_ref,
            startup_ref: startup,
            service_lock_ref: lock,
            supervisor_policy_ref: None,
            heartbeat_count: 0,
            restart_count: 0,
        }
    }

    fn event(name: &str, startup: Option<String>, lock: Option<String>) -> ServiceFsmEvent {
        ServiceFsmEvent {
            event_name: name.to_string(),
            startup_ref: startup,
            service_lock_ref: lock,
            supervisor_policy_ref: Some(local_ref("supervisor-policy")),
            heartbeat_tick: 1,
            max_heartbeat_gap: 2,
            pending_inbox: 0,
            drain_bound: 1,
            max_restarts: 1,
            stale_lock_observed: false,
            duplicate_runner_observed: false,
            shutdown_ref: Some(local_ref("shutdown")),
            authority_refs: refs("authority"),
            policy_refs: refs("policy"),
            resource_refs: refs("resource"),
        }
    }

    // r[verify molten.node_runtime.service_fsm_model]
    // r[verify molten.node_runtime.service_fsm_receipts]
    // r[verify molten.node_runtime.service_fsm_tests]
    #[test]
    fn normal_service_trace_reaches_serving_then_stopped() {
        let startup = local_ref("startup");
        let lock = local_ref("lock");
        let initialized = state(STATE_INITIALIZED, None, None);
        let startup_transition =
            evaluate_service_transition(&initialized, &event(EVENT_STARTUP, Some(startup.clone()), None))
                .expect("startup transition");
        assert_eq!(startup_transition.decision, DECISION_PASS);
        let lock_transition = evaluate_service_transition(
            &startup_transition.next_state,
            &event(EVENT_ACQUIRE_SERVICE_LOCK, Some(startup.clone()), Some(lock.clone())),
        )
        .expect("lock transition");
        assert_eq!(lock_transition.next_state.state_name, STATE_SERVICE_LOCK_HELD);
        assert!(lock_transition.shell_intents.contains(&INTENT_ACQUIRE_LOCK.to_string()));
        let serve = evaluate_service_transition(
            &lock_transition.next_state,
            &event(EVENT_SERVE, Some(startup.clone()), Some(lock.clone())),
        )
        .expect("serve transition");
        assert_eq!(serve.next_state.state_name, STATE_SERVING);
        let shutdown = evaluate_service_transition(
            &serve.next_state,
            &event(EVENT_SHUTDOWN_REQUESTED, Some(startup.clone()), Some(lock.clone())),
        )
        .expect("shutdown transition");
        assert_eq!(shutdown.next_state.state_name, STATE_DRAINING);
        let drained =
            evaluate_service_transition(&shutdown.next_state, &event(EVENT_DRAIN_COMPLETE, Some(startup), Some(lock)))
                .expect("drain transition");
        assert_eq!(drained.next_state.state_name, STATE_STOPPED);
    }

    #[test]
    fn serve_without_startup_denies_without_intents() {
        let state = state(STATE_SERVICE_LOCK_HELD, None, Some(local_ref("lock")));
        let denied = evaluate_service_transition(&state, &event(EVENT_SERVE, None, Some(local_ref("lock"))))
            .expect("denied serve");
        assert_eq!(denied.decision, DECISION_DENY);
        assert!(denied.shell_intents.is_empty());
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic == "missing-startup-evidence"));
    }

    // r[verify molten.node_runtime.service_fsm_lock_recovery]
    #[test]
    fn duplicate_runner_stale_lock_restart_and_drain_denials_preserve_state() {
        let startup = local_ref("startup");
        let lock = local_ref("lock");
        let serving = state(STATE_SERVING, Some(startup.clone()), Some(lock.clone()));
        let duplicate = evaluate_service_transition(
            &serving,
            &event(EVENT_DUPLICATE_RUNNER, Some(startup.clone()), Some(lock.clone())),
        )
        .expect("duplicate");
        assert_eq!(duplicate.decision, DECISION_DENY);
        assert_eq!(duplicate.next_state_ref, serving.state_ref);

        let pending = state(STATE_STALE_LOCK_RECOVERY_PENDING, Some(startup.clone()), Some(lock.clone()));
        let mut stale_recover = event(EVENT_STALE_LOCK_RECOVER, Some(startup.clone()), Some(lock.clone()));
        stale_recover.supervisor_policy_ref = None;
        let stale = evaluate_service_transition(&pending, &stale_recover).expect("stale recovery");
        assert_eq!(stale.decision, DECISION_DENY);
        assert!(stale.diagnostics.iter().any(|diagnostic| diagnostic == "stale-lock-recovery-missing-policy"));

        let mut restart_state = serving.clone();
        restart_state.restart_count = 1;
        let restart = evaluate_service_transition(
            &restart_state,
            &event(EVENT_RESTART_REQUEST, Some(startup.clone()), Some(lock.clone())),
        )
        .expect("restart");
        assert!(restart.diagnostics.iter().any(|diagnostic| diagnostic == "restart-bound-exhausted"));

        let draining = state(STATE_DRAINING, Some(startup.clone()), Some(lock.clone()));
        let mut drain = event(EVENT_DRAIN_COMPLETE, Some(startup), Some(lock));
        drain.pending_inbox = 2;
        drain.drain_bound = 1;
        let drain = evaluate_service_transition(&draining, &drain).expect("drain");
        assert_eq!(drain.decision, DECISION_DENY);
        assert!(drain.diagnostics.iter().any(|diagnostic| diagnostic == "shutdown-drain-bound-exceeded"));
    }
}
