//! End-to-end integration tests for the calendar executor.
//!
//! Tests the full flow: ClientRpcRequest → CalendarServiceExecutor → CalendarStore → KV →
//! ClientRpcResponse

use std::sync::Arc;

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_rpc_core::ServiceExecutor;
use aspen_testing_core::DeterministicKeyValueStore;

fn make_executor() -> aspen_calendar_handler::CalendarServiceExecutor {
    let kv = Arc::new(DeterministicKeyValueStore::new());
    aspen_calendar_handler::CalendarServiceExecutor::new(kv)
}

// ============================================================================
// Task 8.2: End-to-end calendar test
// ============================================================================

#[tokio::test]
async fn test_e2e_create_calendar_add_events_list_freebusy_export() {
    let exec = make_executor();

    // 1. Create a calendar
    let resp = exec
        .execute(ClientRpcRequest::CalendarCreate {
            name: "Work Calendar".to_string(),
            color: None,
            description: Some("Work schedule".to_string()),
            timezone: Some("America/New_York".to_string()),
        })
        .await
        .unwrap();

    let cal_id = match &resp {
        ClientRpcResponse::CalendarResult(r) => {
            assert!(r.is_success, "create calendar failed: {:?}", r.error);
            r.calendar_id.clone().unwrap()
        }
        other => panic!("unexpected response: {other:?}"),
    };

    // 2. Add events via iCal
    let ical1 = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nSUMMARY:Team Standup\r\nDTSTART:20250301T090000Z\r\nDTEND:20250301T093000Z\r\nDESCRIPTION:Daily standup meeting\r\nEND:VEVENT\r\nEND:VCALENDAR";
    let resp = exec
        .execute(ClientRpcRequest::CalendarCreateEvent {
            calendar_id: cal_id.clone(),
            ical_data: ical1.to_string(),
        })
        .await
        .unwrap();

    let event1_id = match &resp {
        ClientRpcResponse::CalendarEventResult(r) => {
            assert!(r.is_success, "create event 1 failed: {:?}", r.error);
            assert!(r.ical_data.as_ref().unwrap().contains("Team Standup"));
            r.event_id.clone().unwrap()
        }
        other => panic!("unexpected response: {other:?}"),
    };

    let ical2 = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nSUMMARY:Lunch Break\r\nDTSTART:20250301T120000Z\r\nDTEND:20250301T130000Z\r\nEND:VEVENT\r\nEND:VCALENDAR";
    let resp = exec
        .execute(ClientRpcRequest::CalendarCreateEvent {
            calendar_id: cal_id.clone(),
            ical_data: ical2.to_string(),
        })
        .await
        .unwrap();

    let _event2_id = match &resp {
        ClientRpcResponse::CalendarEventResult(r) => {
            assert!(r.is_success, "create event 2 failed: {:?}", r.error);
            r.event_id.clone().unwrap()
        }
        other => panic!("unexpected response: {other:?}"),
    };

    // 3. List events by time range (March 1, 2025 full day in UTC ms)
    let start_ms = 1740787200000_u64; // 2025-03-01T00:00:00Z
    let end_ms = 1740873600000_u64; // 2025-03-02T00:00:00Z
    let resp = exec
        .execute(ClientRpcRequest::CalendarListEvents {
            calendar_id: cal_id.clone(),
            start_ms: Some(start_ms),
            end_ms: Some(end_ms),
            limit: None,
            continuation_token: None,
        })
        .await
        .unwrap();

    match &resp {
        ClientRpcResponse::CalendarListEventsResult(r) => {
            assert!(r.is_success, "list events failed: {:?}", r.error);
            assert_eq!(r.total, 2);
            let summaries: Vec<&str> = r.events.iter().map(|e| e.summary.as_str()).collect();
            assert!(summaries.contains(&"Team Standup"), "missing standup in {summaries:?}");
            assert!(summaries.contains(&"Lunch Break"), "missing lunch in {summaries:?}");
        }
        other => panic!("unexpected response: {other:?}"),
    }

    // 4. Free/busy query
    let resp = exec
        .execute(ClientRpcRequest::CalendarFreeBusy {
            calendar_id: cal_id.clone(),
            start_ms: start_ms,
            end_ms: end_ms,
        })
        .await
        .unwrap();

    match &resp {
        ClientRpcResponse::CalendarFreeBusyResult(r) => {
            assert!(r.is_success, "freebusy failed: {:?}", r.error);
            assert_eq!(r.busy_periods.len(), 2, "expected 2 busy periods, got {:?}", r.busy_periods);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    // 5. Export iCal
    let resp = exec
        .execute(ClientRpcRequest::CalendarExportIcal {
            calendar_id: cal_id.clone(),
        })
        .await
        .unwrap();

    let exported_ical = match &resp {
        ClientRpcResponse::CalendarExportResult(r) => {
            assert!(r.is_success, "export failed: {:?}", r.error);
            assert_eq!(r.count, 2);
            let data = r.ical_data.clone().unwrap();
            assert!(data.contains("Team Standup"));
            assert!(data.contains("Lunch Break"));
            data
        }
        other => panic!("unexpected response: {other:?}"),
    };

    // 6. Delete events, import from exported iCal
    exec.execute(ClientRpcRequest::CalendarDeleteEvent { event_id: event1_id }).await.unwrap();

    // Import the exported data
    let resp = exec
        .execute(ClientRpcRequest::CalendarImportIcal {
            calendar_id: cal_id.clone(),
            ical_data: exported_ical,
        })
        .await
        .unwrap();
    match &resp {
        ClientRpcResponse::CalendarExportResult(r) => {
            assert!(r.is_success, "import failed: {:?}", r.error);
            // Imported 2 events from the exported data
            assert!(r.count >= 2, "expected at least 2 imported events, got {}", r.count);
        }
        other => panic!("unexpected response: {other:?}"),
    }
}

// ============================================================================
// Task 8.3: RRULE integration test
// ============================================================================

#[tokio::test]
async fn test_e2e_recurrence_expansion() {
    let exec = make_executor();

    // Create a calendar
    let resp = exec
        .execute(ClientRpcRequest::CalendarCreate {
            name: "Recurring".to_string(),
            color: None,
            description: None,
            timezone: None,
        })
        .await
        .unwrap();
    let cal_id = match &resp {
        ClientRpcResponse::CalendarResult(r) => r.calendar_id.clone().unwrap(),
        other => panic!("unexpected: {other:?}"),
    };

    // Create a daily recurring event with RRULE
    let ical = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nSUMMARY:Daily Standup\r\nDTSTART:20250301T090000Z\r\nDTEND:20250301T093000Z\r\nRRULE:FREQ=DAILY;COUNT=5\r\nEND:VEVENT\r\nEND:VCALENDAR";
    let resp = exec
        .execute(ClientRpcRequest::CalendarCreateEvent {
            calendar_id: cal_id.clone(),
            ical_data: ical.to_string(),
        })
        .await
        .unwrap();

    let event_id = match &resp {
        ClientRpcResponse::CalendarEventResult(r) => {
            assert!(r.is_success, "create recurring event failed: {:?}", r.error);
            r.event_id.clone().unwrap()
        }
        other => panic!("unexpected: {other:?}"),
    };

    // Expand recurrence for March 2025 (full month)
    let range_start_ms = 1740787200000_u64; // 2025-03-01T00:00:00Z
    let range_end_ms = 1743465600000_u64; // 2025-04-01T00:00:00Z
    let resp = exec
        .execute(ClientRpcRequest::CalendarExpandRecurrence {
            event_id: event_id.clone(),
            start_ms: range_start_ms,
            end_ms: range_end_ms,
            max_instances: Some(100),
        })
        .await
        .unwrap();

    match &resp {
        ClientRpcResponse::CalendarExpandResult(r) => {
            assert!(r.is_success, "expand failed: {:?}", r.error);
            // COUNT=5 means 5 instances (Mar 1-5)
            assert_eq!(r.instances.len(), 5, "expected 5 instances, got {}", r.instances.len());
            // Verify instances have valid start times
            for (i, instance) in r.instances.iter().enumerate() {
                assert!(instance.dtstart_ms > 0, "instance {} has zero start time", i);
                if let Some(end) = instance.dtend_ms {
                    assert!(end > instance.dtstart_ms, "instance {} end <= start", i);
                }
            }
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[tokio::test]
async fn test_e2e_recurrence_with_exdate() {
    let exec = make_executor();

    // Create calendar
    let resp = exec
        .execute(ClientRpcRequest::CalendarCreate {
            name: "ExDate Test".to_string(),
            color: None,
            description: None,
            timezone: None,
        })
        .await
        .unwrap();
    let cal_id = match &resp {
        ClientRpcResponse::CalendarResult(r) => r.calendar_id.clone().unwrap(),
        other => panic!("unexpected: {other:?}"),
    };

    // Daily event with COUNT=5 and EXDATE on March 3
    let ical = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nSUMMARY:Exercise\r\nDTSTART:20250301T070000Z\r\nDTEND:20250301T080000Z\r\nRRULE:FREQ=DAILY;COUNT=5\r\nEXDATE:20250303T070000Z\r\nEND:VEVENT\r\nEND:VCALENDAR";
    let resp = exec
        .execute(ClientRpcRequest::CalendarCreateEvent {
            calendar_id: cal_id.clone(),
            ical_data: ical.to_string(),
        })
        .await
        .unwrap();
    let event_id = match &resp {
        ClientRpcResponse::CalendarEventResult(r) => r.event_id.clone().unwrap(),
        other => panic!("unexpected: {other:?}"),
    };

    // Expand
    let range_start_ms = 1740787200000_u64; // 2025-03-01T00:00:00Z
    let range_end_ms = 1743465600000_u64; // 2025-04-01T00:00:00Z
    let resp = exec
        .execute(ClientRpcRequest::CalendarExpandRecurrence {
            event_id,
            start_ms: range_start_ms,
            end_ms: range_end_ms,
            max_instances: Some(100),
        })
        .await
        .unwrap();

    match &resp {
        ClientRpcResponse::CalendarExpandResult(r) => {
            assert!(r.is_success, "expand failed: {:?}", r.error);
            // COUNT=5 minus 1 EXDATE = 4 instances
            assert_eq!(r.instances.len(), 4, "expected 4 instances (5 - 1 EXDATE), got {}", r.instances.len());
        }
        other => panic!("unexpected: {other:?}"),
    }
}

// ============================================================================
// Task 8.2 continued: search and get event
// ============================================================================

#[tokio::test]
async fn test_e2e_search_events() {
    let exec = make_executor();

    // Create calendar
    let resp = exec
        .execute(ClientRpcRequest::CalendarCreate {
            name: "Search Test".to_string(),
            color: None,
            description: None,
            timezone: None,
        })
        .await
        .unwrap();
    let cal_id = match &resp {
        ClientRpcResponse::CalendarResult(r) => r.calendar_id.clone().unwrap(),
        other => panic!("unexpected: {other:?}"),
    };

    // Add events with different summaries
    let events = vec![
        ("Board Meeting", "20250315T140000Z", "20250315T160000Z"),
        ("Team Lunch", "20250315T120000Z", "20250315T130000Z"),
        ("Board Review", "20250316T100000Z", "20250316T120000Z"),
    ];

    for (summary, dtstart, dtend) in &events {
        let ical = format!(
            "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nSUMMARY:{summary}\r\nDTSTART:{dtstart}\r\nDTEND:{dtend}\r\nEND:VEVENT\r\nEND:VCALENDAR"
        );
        let resp = exec
            .execute(ClientRpcRequest::CalendarCreateEvent {
                calendar_id: cal_id.clone(),
                ical_data: ical,
            })
            .await
            .unwrap();
        match &resp {
            ClientRpcResponse::CalendarEventResult(r) => {
                assert!(r.is_success, "create failed for {summary}: {:?}", r.error);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    // Search for "Board"
    let resp = exec
        .execute(ClientRpcRequest::CalendarSearchEvents {
            calendar_id: Some(cal_id.clone()),
            query: "Board".to_string(),
            limit: None,
        })
        .await
        .unwrap();

    match &resp {
        ClientRpcResponse::CalendarSearchResult(r) => {
            assert!(r.is_success, "search failed: {:?}", r.error);
            assert_eq!(r.total, 2, "expected 2 Board events, got {}", r.total);
            let summaries: Vec<&str> = r.events.iter().map(|e| e.summary.as_str()).collect();
            assert!(summaries.contains(&"Board Meeting"));
            assert!(summaries.contains(&"Board Review"));
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[tokio::test]
async fn test_e2e_get_update_delete_event() {
    let exec = make_executor();

    // Create calendar
    let resp = exec
        .execute(ClientRpcRequest::CalendarCreate {
            name: "CRUD Test".to_string(),
            color: None,
            description: None,
            timezone: None,
        })
        .await
        .unwrap();
    let cal_id = match &resp {
        ClientRpcResponse::CalendarResult(r) => r.calendar_id.clone().unwrap(),
        other => panic!("unexpected: {other:?}"),
    };

    // Create event
    let ical = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nSUMMARY:Original Event\r\nDTSTART:20250401T100000Z\r\nDTEND:20250401T110000Z\r\nEND:VEVENT\r\nEND:VCALENDAR";
    let resp = exec
        .execute(ClientRpcRequest::CalendarCreateEvent {
            calendar_id: cal_id.clone(),
            ical_data: ical.to_string(),
        })
        .await
        .unwrap();
    let event_id = match &resp {
        ClientRpcResponse::CalendarEventResult(r) => r.event_id.clone().unwrap(),
        other => panic!("unexpected: {other:?}"),
    };

    // Get event
    let resp = exec
        .execute(ClientRpcRequest::CalendarGetEvent {
            event_id: event_id.clone(),
        })
        .await
        .unwrap();
    match &resp {
        ClientRpcResponse::CalendarEventResult(r) => {
            assert!(r.is_success);
            assert!(r.ical_data.as_ref().unwrap().contains("Original Event"));
        }
        other => panic!("unexpected: {other:?}"),
    }

    // Update event
    let updated_ical = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nSUMMARY:Updated Event\r\nDTSTART:20250401T140000Z\r\nDTEND:20250401T150000Z\r\nDESCRIPTION:Rescheduled\r\nEND:VEVENT\r\nEND:VCALENDAR";
    let resp = exec
        .execute(ClientRpcRequest::CalendarUpdateEvent {
            event_id: event_id.clone(),
            ical_data: updated_ical.to_string(),
        })
        .await
        .unwrap();
    match &resp {
        ClientRpcResponse::CalendarEventResult(r) => {
            assert!(r.is_success, "update failed: {:?}", r.error);
            assert!(r.ical_data.as_ref().unwrap().contains("Updated Event"));
        }
        other => panic!("unexpected: {other:?}"),
    }

    // Verify update persisted
    let resp = exec
        .execute(ClientRpcRequest::CalendarGetEvent {
            event_id: event_id.clone(),
        })
        .await
        .unwrap();
    match &resp {
        ClientRpcResponse::CalendarEventResult(r) => {
            assert!(r.is_success);
            let data = r.ical_data.as_ref().unwrap();
            assert!(data.contains("Updated Event"));
            assert!(!data.contains("Original Event"));
        }
        other => panic!("unexpected: {other:?}"),
    }

    // Delete event
    let resp = exec
        .execute(ClientRpcRequest::CalendarDeleteEvent {
            event_id: event_id.clone(),
        })
        .await
        .unwrap();
    match &resp {
        ClientRpcResponse::CalendarEventResult(r) => {
            assert!(r.is_success);
        }
        other => panic!("unexpected: {other:?}"),
    }

    // Verify deletion
    let resp = exec
        .execute(ClientRpcRequest::CalendarGetEvent {
            event_id: event_id.clone(),
        })
        .await
        .unwrap();
    match &resp {
        ClientRpcResponse::CalendarEventResult(r) => {
            assert!(!r.is_success, "expected get to fail after deletion");
            assert!(r.error.is_some());
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[tokio::test]
async fn test_e2e_list_calendars_and_delete() {
    let exec = make_executor();

    // Create multiple calendars
    exec.execute(ClientRpcRequest::CalendarCreate {
        name: "Cal A".to_string(),
        color: None,
        description: None,
        timezone: None,
    })
    .await
    .unwrap();

    exec.execute(ClientRpcRequest::CalendarCreate {
        name: "Cal B".to_string(),
        color: None,
        description: Some("Second calendar".to_string()),
        timezone: Some("UTC".to_string()),
    })
    .await
    .unwrap();

    // List calendars
    let resp = exec.execute(ClientRpcRequest::CalendarList { limit: None }).await.unwrap();
    let (cal1_id, cal2_id) = match &resp {
        ClientRpcResponse::CalendarListResult(r) => {
            assert!(r.is_success, "list failed: {:?}", r.error);
            assert_eq!(r.calendars.len(), 2);
            (r.calendars[0].id.clone(), r.calendars[1].id.clone())
        }
        other => panic!("unexpected: {other:?}"),
    };

    // Delete first calendar
    let resp = exec
        .execute(ClientRpcRequest::CalendarDelete {
            calendar_id: cal1_id.clone(),
        })
        .await
        .unwrap();
    match &resp {
        ClientRpcResponse::CalendarResult(r) => assert!(r.is_success),
        other => panic!("unexpected: {other:?}"),
    }

    // Verify only one remains
    let resp = exec.execute(ClientRpcRequest::CalendarList { limit: None }).await.unwrap();
    match &resp {
        ClientRpcResponse::CalendarListResult(r) => {
            assert!(r.is_success);
            assert_eq!(r.calendars.len(), 1);
            assert_eq!(r.calendars[0].id, cal2_id);
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[tokio::test]
async fn test_e2e_import_multi_event_ical() {
    let exec = make_executor();

    // Create calendar
    let resp = exec
        .execute(ClientRpcRequest::CalendarCreate {
            name: "Import Test".to_string(),
            color: None,
            description: None,
            timezone: None,
        })
        .await
        .unwrap();
    let cal_id = match &resp {
        ClientRpcResponse::CalendarResult(r) => r.calendar_id.clone().unwrap(),
        other => panic!("unexpected: {other:?}"),
    };

    // Import iCal with multiple events
    let multi_ical = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nSUMMARY:Event One\r\nDTSTART:20250501T090000Z\r\nDTEND:20250501T100000Z\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nSUMMARY:Event Two\r\nDTSTART:20250502T090000Z\r\nDTEND:20250502T100000Z\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nSUMMARY:Event Three\r\nDTSTART:20250503T090000Z\r\nDTEND:20250503T100000Z\r\nEND:VEVENT\r\nEND:VCALENDAR";

    let resp = exec
        .execute(ClientRpcRequest::CalendarImportIcal {
            calendar_id: cal_id.clone(),
            ical_data: multi_ical.to_string(),
        })
        .await
        .unwrap();
    match &resp {
        ClientRpcResponse::CalendarExportResult(r) => {
            assert!(r.is_success, "import failed: {:?}", r.error);
            assert_eq!(r.count, 3, "expected 3 imported events");
        }
        other => panic!("unexpected: {other:?}"),
    }

    // Verify all 3 events exist
    let start_ms = 1746057600000_u64; // 2025-05-01T00:00:00Z
    let end_ms = 1746316800000_u64; // 2025-05-04T00:00:00Z
    let resp = exec
        .execute(ClientRpcRequest::CalendarListEvents {
            calendar_id: cal_id.clone(),
            start_ms: Some(start_ms),
            end_ms: Some(end_ms),
            limit: None,
            continuation_token: None,
        })
        .await
        .unwrap();
    match &resp {
        ClientRpcResponse::CalendarListEventsResult(r) => {
            assert!(r.is_success);
            assert_eq!(r.total, 3);
        }
        other => panic!("unexpected: {other:?}"),
    }
}
