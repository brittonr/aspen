//! Tests for the event store module.

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use aspen_hlc::SerializableTimestamp;
    use aspen_hlc::create_hlc;
    use chrono::Utc;

    use crate::event_store::WorkflowEvent;
    use crate::event_store::WorkflowEventStore;
    use crate::event_store::event_types::WorkflowEventType;
    use crate::event_store::replay::WorkflowReplayEngine;
    use crate::event_store::snapshot::WorkflowSnapshot;
    use crate::event_store::types::EVENT_SCHEMA_VERSION;
    use crate::event_store::types::WorkflowExecutionId;
    use crate::event_store::upcast_event;

    #[test]
    fn test_workflow_execution_id() {
        let id = WorkflowExecutionId::new();
        assert!(!id.as_str().is_empty());

        let id2 = WorkflowExecutionId::from_string("test-123".to_string());
        assert_eq!(id2.as_str(), "test-123");
    }

    #[test]
    fn test_event_storage_key() {
        let workflow_id = WorkflowExecutionId::from_string("wf-123".to_string());
        let hlc = create_hlc("test-node");
        let event = WorkflowEvent::new(
            42,
            workflow_id.clone(),
            WorkflowEventType::WorkflowStarted {
                workflow_type: "test".to_string(),
                input: serde_json::json!({}),
                parent_workflow_id: None,
            },
            &hlc,
            Some(41),
        );

        let key = event.storage_key();
        assert!(key.starts_with("__wf_events::wf-123::"));
        assert!(key.contains("00000000000000000042"));
    }

    #[test]
    fn test_snapshot_storage_key() {
        let workflow_id = WorkflowExecutionId::from_string("wf-456".to_string());
        let hlc = create_hlc("test-node");
        let snapshot = WorkflowSnapshot {
            schema_version: 1,
            snapshot_id: 5,
            workflow_id: workflow_id.clone(),
            at_event_id: 1000,
            hlc_timestamp: SerializableTimestamp::from(hlc.new_timestamp()),
            timestamp: Utc::now(),
            state: serde_json::json!({"counter": 42}),
            pending_activities: HashMap::new(),
            pending_timers: HashMap::new(),
            compensation_stack: Vec::new(),
            side_effect_seq: 10,
        };

        let key = snapshot.storage_key();
        assert!(key.starts_with("__wf_snapshots::wf-456::"));
        assert!(key.contains("00000000000000000005"));
    }

    #[test]
    fn test_replay_engine_memoization() {
        let workflow_id = WorkflowExecutionId::new();
        let hlc = create_hlc("test-node");

        let events = vec![
            WorkflowEvent::new(
                0,
                workflow_id.clone(),
                WorkflowEventType::WorkflowStarted {
                    workflow_type: "test".to_string(),
                    input: serde_json::json!({}),
                    parent_workflow_id: None,
                },
                &hlc,
                None,
            ),
            WorkflowEvent::new(
                1,
                workflow_id.clone(),
                WorkflowEventType::ActivityScheduled {
                    activity_id: "act-1".to_string(),
                    activity_type: "fetch_data".to_string(),
                    input: serde_json::json!({"url": "http://example.com"}),
                    timeout_ms: Some(30000),
                },
                &hlc,
                Some(0),
            ),
            WorkflowEvent::new(
                2,
                workflow_id.clone(),
                WorkflowEventType::ActivityCompleted {
                    activity_id: "act-1".to_string(),
                    result: serde_json::json!({"data": "cached_result"}),
                    duration_ms: 150,
                },
                &hlc,
                Some(1),
            ),
            WorkflowEvent::new(
                3,
                workflow_id.clone(),
                WorkflowEventType::SideEffectRecorded {
                    seq: 0,
                    value: serde_json::json!("uuid-12345"),
                },
                &hlc,
                Some(2),
            ),
        ];

        let mut engine = WorkflowReplayEngine::new(events);

        // Should have cached activity result
        let cached = engine.get_activity_result("act-1");
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), &serde_json::json!({"data": "cached_result"}));

        // Should have cached side effect
        let side_effect = engine.get_side_effect();
        assert!(side_effect.is_some());
        assert_eq!(side_effect.unwrap(), serde_json::json!("uuid-12345"));

        // Is replaying should be true initially
        assert!(engine.is_replaying());
    }

    #[test]
    fn test_should_snapshot() {
        let store = aspen_testing::DeterministicKeyValueStore::new();
        let event_store = WorkflowEventStore::new(Arc::new(store), "test-node".to_string());

        // No previous snapshot, just started
        assert!(!event_store.should_snapshot(100, None));
        assert!(event_store.should_snapshot(1000, None));
        assert!(event_store.should_snapshot(1500, None));

        // With previous snapshot
        assert!(!event_store.should_snapshot(1100, Some(1000)));
        assert!(event_store.should_snapshot(2000, Some(1000)));
    }

    // ============================================================================
    // Event Schema Migration (Upcasting) Tests
    // ============================================================================

    #[test]
    fn test_upcast_event_current_version() {
        // Version 1 is the current version - should pass through unchanged
        let workflow_id = WorkflowExecutionId::from_string("wf-test".to_string());
        let hlc = create_hlc("test-node");
        let event = WorkflowEvent::new(
            0,
            workflow_id,
            WorkflowEventType::WorkflowStarted {
                workflow_type: "test_workflow".to_string(),
                input: serde_json::json!({"key": "value"}),
                parent_workflow_id: None,
            },
            &hlc,
            None,
        );

        assert_eq!(event.schema_version, EVENT_SCHEMA_VERSION);
        let result = upcast_event(event.clone());
        assert!(result.is_ok());
        let migrated = result.unwrap();
        assert_eq!(migrated.schema_version, EVENT_SCHEMA_VERSION);
        assert_eq!(migrated.event_id, 0);
    }

    #[test]
    fn test_upcast_event_version_0_rejected() {
        // Version 0 was never released - should be rejected
        let workflow_id = WorkflowExecutionId::from_string("wf-test".to_string());
        let hlc = create_hlc("test-node");
        let mut event = WorkflowEvent::new(
            42,
            workflow_id,
            WorkflowEventType::WorkflowCompleted {
                output: serde_json::json!({"result": "success"}),
            },
            &hlc,
            Some(41),
        );
        event.schema_version = 0;

        let result = upcast_event(event);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("unknown event schema version 0"));
        assert!(error.contains("event_id 42"));
    }

    #[test]
    fn test_upcast_event_future_version_rejected() {
        // Future versions should be rejected with upgrade message
        let workflow_id = WorkflowExecutionId::from_string("wf-test".to_string());
        let hlc = create_hlc("test-node");
        let mut event = WorkflowEvent::new(
            100,
            workflow_id,
            WorkflowEventType::ActivityScheduled {
                activity_id: "act-1".to_string(),
                activity_type: "fetch".to_string(),
                input: serde_json::json!({}),
                timeout_ms: Some(5000),
            },
            &hlc,
            Some(99),
        );
        event.schema_version = 999; // Far future version

        let result = upcast_event(event);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("event schema version 999"));
        assert!(error.contains("newer than supported"));
        assert!(error.contains("upgrade aspen-jobs"));
    }

    #[test]
    fn test_upcast_event_unknown_version_rejected() {
        // Unknown intermediate versions should be rejected
        let workflow_id = WorkflowExecutionId::from_string("wf-test".to_string());
        let hlc = create_hlc("test-node");
        let mut event = WorkflowEvent::new(
            5,
            workflow_id,
            WorkflowEventType::TimerFired {
                timer_id: "timer-1".to_string(),
            },
            &hlc,
            Some(4),
        );
        // Set to a version between 0 and current that doesn't exist
        // Since current is 1 and 0 is handled specially, we need to use
        // a version > current to test the "newer" path
        event.schema_version = 2;

        let result = upcast_event(event);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("newer than supported"));
    }

    #[test]
    fn test_upcast_preserves_all_event_fields() {
        // Ensure upcasting preserves all event data
        let workflow_id = WorkflowExecutionId::from_string("wf-preserve".to_string());
        let hlc = create_hlc("test-node");
        let event = WorkflowEvent::new(
            42,
            workflow_id.clone(),
            WorkflowEventType::ActivityCompleted {
                activity_id: "act-preserve".to_string(),
                result: serde_json::json!({"nested": {"data": [1, 2, 3]}}),
                duration_ms: 12345,
            },
            &hlc,
            Some(41),
        );

        let original_timestamp = event.timestamp;
        let original_hlc = event.hlc_timestamp.clone();

        let result = upcast_event(event);
        assert!(result.is_ok());
        let migrated = result.unwrap();

        // Verify all fields are preserved
        assert_eq!(migrated.event_id, 42);
        assert_eq!(migrated.workflow_id.as_str(), "wf-preserve");
        assert_eq!(migrated.timestamp, original_timestamp);
        assert_eq!(migrated.hlc_timestamp, original_hlc);
        assert_eq!(migrated.prev_event_id, Some(41));

        // Verify event type data is preserved
        if let WorkflowEventType::ActivityCompleted {
            activity_id,
            result,
            duration_ms,
        } = migrated.event_type
        {
            assert_eq!(activity_id, "act-preserve");
            assert_eq!(result, serde_json::json!({"nested": {"data": [1, 2, 3]}}));
            assert_eq!(duration_ms, 12345);
        } else {
            panic!("Event type was not preserved correctly");
        }
    }

    #[test]
    fn test_upcast_all_event_types() {
        // Test that upcasting works for all WorkflowEventType variants
        let workflow_id = WorkflowExecutionId::from_string("wf-types".to_string());
        let hlc = create_hlc("test-node");

        let event_types = vec![
            WorkflowEventType::WorkflowStarted {
                workflow_type: "test".to_string(),
                input: serde_json::json!({}),
                parent_workflow_id: None,
            },
            WorkflowEventType::WorkflowCompleted {
                output: serde_json::json!({}),
            },
            WorkflowEventType::WorkflowFailed {
                error: "test error".to_string(),
                error_type: Some("TestError".to_string()),
                is_retryable: true,
            },
            WorkflowEventType::WorkflowCancelled {
                reason: "user request".to_string(),
            },
            WorkflowEventType::WorkflowTimedOut {
                timeout_type: "execution".to_string(),
            },
            WorkflowEventType::ActivityScheduled {
                activity_id: "a1".to_string(),
                activity_type: "fetch".to_string(),
                input: serde_json::json!({}),
                timeout_ms: None,
            },
            WorkflowEventType::ActivityStarted {
                activity_id: "a1".to_string(),
                worker_id: "w1".to_string(),
                attempt: 1,
            },
            WorkflowEventType::ActivityCompleted {
                activity_id: "a1".to_string(),
                result: serde_json::json!({}),
                duration_ms: 100,
            },
            WorkflowEventType::ActivityFailed {
                activity_id: "a1".to_string(),
                error: "failed".to_string(),
                is_retryable: true,
                attempt: 1,
            },
            WorkflowEventType::ActivityRetrying {
                activity_id: "a1".to_string(),
                attempt: 2,
                retry_delay_ms: 1000,
            },
            WorkflowEventType::ActivityCancelled {
                activity_id: "a1".to_string(),
                reason: "timeout".to_string(),
            },
            WorkflowEventType::TimerScheduled {
                timer_id: "t1".to_string(),
                fire_at: Utc::now(),
                duration_ms: 5000,
            },
            WorkflowEventType::TimerFired {
                timer_id: "t1".to_string(),
            },
            WorkflowEventType::TimerCancelled {
                timer_id: "t1".to_string(),
            },
            WorkflowEventType::SignalReceived {
                signal_name: "my_signal".to_string(),
                payload: serde_json::json!({"data": "test"}),
            },
            WorkflowEventType::StateTransitioned {
                from_state: "pending".to_string(),
                to_state: "running".to_string(),
                trigger: "start".to_string(),
            },
            WorkflowEventType::ChildWorkflowStarted {
                child_workflow_id: WorkflowExecutionId::new(),
                workflow_type: "child_type".to_string(),
                input: serde_json::json!({}),
            },
            WorkflowEventType::ChildWorkflowCompleted {
                child_workflow_id: WorkflowExecutionId::new(),
                result: serde_json::json!({}),
            },
            WorkflowEventType::ChildWorkflowFailed {
                child_workflow_id: WorkflowExecutionId::new(),
                error: "child failed".to_string(),
            },
            WorkflowEventType::CompensationRegistered {
                compensation_id: "c1".to_string(),
                compensation_activity: "undo_action".to_string(),
                compensation_input: serde_json::json!({}),
            },
            WorkflowEventType::CompensationStarted {
                compensation_id: "c1".to_string(),
            },
            WorkflowEventType::CompensationCompleted {
                compensation_id: "c1".to_string(),
            },
            WorkflowEventType::CompensationFailed {
                compensation_id: "c1".to_string(),
                error: "compensation failed".to_string(),
            },
            WorkflowEventType::SnapshotTaken {
                snapshot_id: 1,
                at_event_id: 100,
            },
            WorkflowEventType::ContinuedAsNew {
                new_workflow_id: WorkflowExecutionId::new(),
                input: serde_json::json!({}),
            },
            WorkflowEventType::SideEffectRecorded {
                seq: 0,
                value: serde_json::json!("uuid-123"),
            },
            WorkflowEventType::VersionMarker {
                patch_id: "v2_feature".to_string(),
                new_path: true,
            },
        ];

        for (i, event_type) in event_types.into_iter().enumerate() {
            let event = WorkflowEvent::new(
                i as u64,
                workflow_id.clone(),
                event_type,
                &hlc,
                if i > 0 { Some((i - 1) as u64) } else { None },
            );

            let result = upcast_event(event);
            assert!(result.is_ok(), "Upcast failed for event type at index {}: {:?}", i, result.err());
        }
    }

    #[test]
    fn test_event_schema_version_constant() {
        // Ensure the schema version constant is what we expect
        assert_eq!(EVENT_SCHEMA_VERSION, 1, "Current schema version should be 1");
    }

    #[test]
    fn test_new_event_has_current_schema_version() {
        // Ensure newly created events have the current schema version
        let workflow_id = WorkflowExecutionId::new();
        let hlc = create_hlc("test-node");
        let event = WorkflowEvent::new(
            0,
            workflow_id,
            WorkflowEventType::WorkflowStarted {
                workflow_type: "test".to_string(),
                input: serde_json::json!({}),
                parent_workflow_id: None,
            },
            &hlc,
            None,
        );

        assert_eq!(event.schema_version, EVENT_SCHEMA_VERSION);
    }
}
