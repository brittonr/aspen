//! End-to-end integration tests for calendar.
//!
//! Tests the full calendar workflow: create calendar → add events → list by
//! time range → free/busy query → export/import iCal roundtrip → RRULE expansion.

use std::sync::Arc;

use aspen_calendar::CalendarStore;
use aspen_testing_core::DeterministicKeyValueStore;

async fn store() -> (CalendarStore<DeterministicKeyValueStore>, Arc<DeterministicKeyValueStore>) {
    let kv = DeterministicKeyValueStore::new();
    let cs = CalendarStore::new(kv.clone());
    (cs, kv)
}

const DAY_MS: u64 = 86_400_000;
const HOUR_MS: u64 = 3_600_000;

// Known iCal datetimes and their Unix-ms equivalents.
// 2023-11-14 09:00 UTC = 20231114T090000Z = 1699952400000
const BASE_MS: u64 = 1_699_952_400_000;
const BASE_ICAL: &str = "20231114T090000Z";

/// Offset hours from the base time as iCal datetime string.
fn ical_dt(offset_hours: u32) -> String {
    // Just compute real iCal datetimes via simple hour math.
    let total_h = 9 + offset_hours;
    let day = 14 + total_h / 24;
    let hour = total_h % 24;
    format!("202311{day:02}T{hour:02}0000Z")
}

/// Offset hours from base as Unix-ms.
fn ms_at(offset_hours: u64) -> u64 {
    BASE_MS + offset_hours * HOUR_MS
}

// =========================================================================
// 8.2  End-to-end calendar
// =========================================================================

#[tokio::test]
async fn e2e_calendar_create_events_list_freebusy_export_roundtrip() {
    let (cs, _) = store().await;
    let now = BASE_MS;

    // 1. Create a calendar.
    let cal = cs
        .create_calendar("Work", Some("#4285f4"), Some("America/New_York"), Some("Work calendar"), now)
        .await
        .unwrap();
    assert!(!cal.id.is_empty());
    assert_eq!(cal.name, "Work");
    assert_eq!(cal.color.as_deref(), Some("#4285f4"));

    // 2. Add events using proper iCal datetime format.
    // Meeting: 10:00-11:00 on Nov 14
    let vevent_meeting = format!(
        "BEGIN:VEVENT\r\nSUMMARY:Team Meeting\r\nDTSTART:{}\r\nDTEND:{}\r\nLOCATION:Room A\r\nEND:VEVENT\r\n",
        ical_dt(1),
        ical_dt(2)
    );
    // Lunch: 13:00-14:00 on Nov 14
    let vevent_lunch = format!(
        "BEGIN:VEVENT\r\nSUMMARY:Lunch Break\r\nDTSTART:{}\r\nDTEND:{}\r\nEND:VEVENT\r\n",
        ical_dt(4),
        ical_dt(5)
    );
    // Tomorrow: 09:00-10:00 on Nov 15 (24h offset)
    let vevent_tomorrow = format!(
        "BEGIN:VEVENT\r\nSUMMARY:Tomorrow Task\r\nDTSTART:{}\r\nDTEND:{}\r\nDESCRIPTION:A task for tomorrow\r\nEND:VEVENT\r\n",
        ical_dt(24),
        ical_dt(25)
    );

    let meeting = cs.create_event(&cal.id, &vevent_meeting, now).await.unwrap();
    let lunch = cs.create_event(&cal.id, &vevent_lunch, now + 1).await.unwrap();
    let tomorrow = cs.create_event(&cal.id, &vevent_tomorrow, now + 2).await.unwrap();

    assert_eq!(meeting.summary, "Team Meeting");
    assert_eq!(lunch.summary, "Lunch Break");
    assert_eq!(tomorrow.summary, "Tomorrow Task");

    // Verify dtstart_ms was parsed correctly.
    assert_eq!(meeting.dtstart_ms, ms_at(1));
    assert_eq!(lunch.dtstart_ms, ms_at(4));
    assert_eq!(tomorrow.dtstart_ms, ms_at(24));

    // 3. List all events (no time filter).
    let (events, _token) = cs.list_events(&cal.id, None, None, None).await.unwrap();
    assert_eq!(events.len(), 3);

    // 4. List events in today's range only (09:00 to just before next day 09:00).
    // Note: event_in_range uses `dtstart > end` (inclusive end), so subtract 1ms.
    let (today_events, _) = cs.list_events(&cal.id, Some(ms_at(0)), Some(ms_at(24) - 1), None).await.unwrap();
    assert_eq!(today_events.len(), 2, "expected only meeting and lunch within today");
    let names: Vec<_> = today_events.iter().map(|e| e.summary.as_str()).collect();
    assert!(names.contains(&"Team Meeting"));
    assert!(names.contains(&"Lunch Break"));

    // 5. Free/busy query over today.
    let busy = cs.free_busy_query(&cal.id, ms_at(0), ms_at(24)).await.unwrap();
    assert_eq!(busy.len(), 2);

    // 6. Get single event.
    let got = cs.get_event(&meeting.id).await.unwrap();
    assert_eq!(got.summary, "Team Meeting");
    assert_eq!(got.location.as_deref(), Some("Room A"));

    // 7. Export iCal.
    let (exported, count) = cs.export_ical(&cal.id).await.unwrap();
    assert_eq!(count, 3);
    assert!(exported.contains("Team Meeting"));
    assert!(exported.contains("Lunch Break"));
    assert!(exported.contains("Tomorrow Task"));

    // 8. Delete an event.
    cs.delete_event(&lunch.id).await.unwrap();
    let (events, _) = cs.list_events(&cal.id, None, None, None).await.unwrap();
    assert_eq!(events.len(), 2);

    // 9. Update an event.
    let updated_ical = format!(
        "BEGIN:VEVENT\r\nSUMMARY:Updated Meeting\r\nDTSTART:{}\r\nDTEND:{}\r\nEND:VEVENT\r\n",
        ical_dt(1),
        ical_dt(3)
    );
    let updated = cs.update_event(&meeting.id, &updated_ical, now + 10).await.unwrap();
    assert_eq!(updated.summary, "Updated Meeting");
}

#[tokio::test]
async fn e2e_calendar_search_events() {
    let (cs, _) = store().await;
    let now = BASE_MS;

    let cal = cs.create_calendar("Search Test", None, None, None, now).await.unwrap();

    cs.create_event(
        &cal.id,
        &format!("BEGIN:VEVENT\r\nSUMMARY:Sprint Planning\r\nDTSTART:{}\r\nEND:VEVENT\r\n", ical_dt(0)),
        now,
    )
    .await
    .unwrap();
    cs.create_event(
        &cal.id,
        &format!("BEGIN:VEVENT\r\nSUMMARY:Sprint Retro\r\nDTSTART:{}\r\nEND:VEVENT\r\n", ical_dt(1)),
        now + 1,
    )
    .await
    .unwrap();
    cs.create_event(
        &cal.id,
        &format!("BEGIN:VEVENT\r\nSUMMARY:Lunch\r\nDTSTART:{}\r\nEND:VEVENT\r\n", ical_dt(2)),
        now + 2,
    )
    .await
    .unwrap();

    // Search for "sprint" should find 2 events.
    let results = cs.search_events("sprint", Some(&cal.id), None).await.unwrap();
    assert_eq!(results.len(), 2);

    // Search for "lunch" should find 1 event.
    let results = cs.search_events("lunch", Some(&cal.id), None).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].summary, "Lunch");
}

#[tokio::test]
async fn e2e_calendar_import_ical_bulk() {
    let (cs, _) = store().await;
    let now = BASE_MS;

    let cal = cs.create_calendar("Imported", None, None, None, now).await.unwrap();

    let ical_data = format!(
        "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//test\r\n\
         BEGIN:VEVENT\r\nSUMMARY:Event One\r\nDTSTART:{}\r\nEND:VEVENT\r\n\
         BEGIN:VEVENT\r\nSUMMARY:Event Two\r\nDTSTART:{}\r\nEND:VEVENT\r\n\
         BEGIN:VEVENT\r\nSUMMARY:Event Three\r\nDTSTART:{}\r\nEND:VEVENT\r\n\
         END:VCALENDAR\r\n",
        ical_dt(0),
        ical_dt(1),
        ical_dt(2)
    );

    let imported = cs.import_ical(&cal.id, &ical_data, now).await.unwrap();
    assert_eq!(imported.len(), 3);

    let (events, _) = cs.list_events(&cal.id, None, None, None).await.unwrap();
    assert_eq!(events.len(), 3);
}

#[tokio::test]
async fn e2e_calendar_multiple_calendars() {
    let (cs, _) = store().await;
    let now = BASE_MS;

    let work = cs.create_calendar("Work", None, None, None, now).await.unwrap();
    let personal = cs.create_calendar("Personal", None, None, None, now).await.unwrap();

    cs.create_event(
        &work.id,
        &format!("BEGIN:VEVENT\r\nSUMMARY:Work Event\r\nDTSTART:{}\r\nEND:VEVENT\r\n", ical_dt(0)),
        now,
    )
    .await
    .unwrap();
    cs.create_event(
        &personal.id,
        &format!("BEGIN:VEVENT\r\nSUMMARY:Personal Event\r\nDTSTART:{}\r\nEND:VEVENT\r\n", ical_dt(1)),
        now + 1,
    )
    .await
    .unwrap();

    let (work_events, _) = cs.list_events(&work.id, None, None, None).await.unwrap();
    assert_eq!(work_events.len(), 1);
    assert_eq!(work_events[0].summary, "Work Event");

    let (personal_events, _) = cs.list_events(&personal.id, None, None, None).await.unwrap();
    assert_eq!(personal_events.len(), 1);
    assert_eq!(personal_events[0].summary, "Personal Event");

    let cals = cs.list_calendars(None).await.unwrap();
    assert_eq!(cals.len(), 2);
}

// =========================================================================
// 8.3  RRULE integration test
// =========================================================================

#[tokio::test]
async fn e2e_rrule_daily_recurrence_expansion() {
    let (cs, _) = store().await;
    let now = BASE_MS;

    let cal = cs.create_calendar("RRULE Test", None, None, None, now).await.unwrap();

    // Create a daily recurring event starting at 09:00 Nov 14, 1-hour duration.
    let vevent = format!(
        "BEGIN:VEVENT\r\nSUMMARY:Daily Standup\r\nDTSTART:{}\r\nDTEND:{}\r\nRRULE:FREQ=DAILY;COUNT=7\r\nEND:VEVENT\r\n",
        ical_dt(0),
        ical_dt(1)
    );
    let event = cs.create_event(&cal.id, &vevent, now).await.unwrap();
    assert_eq!(event.summary, "Daily Standup");
    assert_eq!(event.rrule.as_deref(), Some("FREQ=DAILY;COUNT=7"));
    assert_eq!(event.dtstart_ms, ms_at(0));

    // Expand recurrence over a 2-week window.
    let instances = cs.expand_recurrence(&event.id, ms_at(0), ms_at(0) + 14 * DAY_MS, None).await.unwrap();

    // COUNT=7 → 7 instances.
    assert_eq!(instances.len(), 7, "expected 7 instances, got {}", instances.len());

    // Verify first instance starts at dtstart.
    assert_eq!(instances[0].dtstart_ms, ms_at(0));
    assert_eq!(instances[0].dtend_ms, Some(ms_at(1)));

    // Verify second instance is one day later.
    assert_eq!(instances[1].dtstart_ms, ms_at(0) + DAY_MS);

    // Verify last instance.
    assert_eq!(instances[6].dtstart_ms, ms_at(0) + 6 * DAY_MS);
}

#[tokio::test]
async fn e2e_rrule_weekly_recurrence_expansion() {
    let (cs, _) = store().await;
    let now = BASE_MS;

    let cal = cs.create_calendar("Weekly Test", None, None, None, now).await.unwrap();

    let vevent = format!(
        "BEGIN:VEVENT\r\nSUMMARY:Weekly Review\r\nDTSTART:{}\r\nDTEND:{}\r\nRRULE:FREQ=WEEKLY;COUNT=4\r\nEND:VEVENT\r\n",
        ical_dt(0),
        ical_dt(2)
    );
    let event = cs.create_event(&cal.id, &vevent, now).await.unwrap();

    let instances = cs.expand_recurrence(&event.id, ms_at(0), ms_at(0) + 30 * DAY_MS, None).await.unwrap();
    assert_eq!(instances.len(), 4);

    // Each instance is 7 days apart.
    for i in 0..4 {
        assert_eq!(instances[i].dtstart_ms, ms_at(0) + i as u64 * 7 * DAY_MS);
    }
}

#[tokio::test]
async fn e2e_rrule_no_rrule_returns_empty() {
    let (cs, _) = store().await;
    let now = BASE_MS;

    let cal = cs.create_calendar("No RRULE", None, None, None, now).await.unwrap();

    let vevent = format!("BEGIN:VEVENT\r\nSUMMARY:Single Event\r\nDTSTART:{}\r\nEND:VEVENT\r\n", ical_dt(0));
    let event = cs.create_event(&cal.id, &vevent, now).await.unwrap();
    assert!(event.rrule.is_none());

    let instances = cs.expand_recurrence(&event.id, ms_at(0), ms_at(0) + 30 * DAY_MS, None).await.unwrap();
    assert!(instances.is_empty(), "non-recurring event should produce 0 instances");
}

#[tokio::test]
async fn e2e_rrule_max_instances_limit() {
    let (cs, _) = store().await;
    let now = BASE_MS;

    let cal = cs.create_calendar("Max Inst", None, None, None, now).await.unwrap();

    // Create a daily event with no COUNT (unlimited).
    let vevent = format!(
        "BEGIN:VEVENT\r\nSUMMARY:Daily Unlimited\r\nDTSTART:{}\r\nDTEND:{}\r\nRRULE:FREQ=DAILY\r\nEND:VEVENT\r\n",
        ical_dt(0),
        ical_dt(1)
    );
    let event = cs.create_event(&cal.id, &vevent, now).await.unwrap();

    // Expand with max 5 instances.
    let instances = cs.expand_recurrence(&event.id, ms_at(0), ms_at(0) + 365 * DAY_MS, Some(5)).await.unwrap();
    assert_eq!(instances.len(), 5, "max_instances should cap at 5");
}

#[tokio::test]
async fn e2e_free_busy_with_multiple_events() {
    let (cs, _) = store().await;
    let now = BASE_MS;

    let cal = cs.create_calendar("FB Test", None, None, None, now).await.unwrap();

    // Two events today, one tomorrow.
    cs.create_event(
        &cal.id,
        &format!(
            "BEGIN:VEVENT\r\nSUMMARY:Morning\r\nDTSTART:{}\r\nDTEND:{}\r\nEND:VEVENT\r\n",
            ical_dt(0),
            ical_dt(1)
        ),
        now,
    )
    .await
    .unwrap();
    cs.create_event(
        &cal.id,
        &format!(
            "BEGIN:VEVENT\r\nSUMMARY:Afternoon\r\nDTSTART:{}\r\nDTEND:{}\r\nEND:VEVENT\r\n",
            ical_dt(4),
            ical_dt(5)
        ),
        now + 1,
    )
    .await
    .unwrap();
    cs.create_event(
        &cal.id,
        &format!(
            "BEGIN:VEVENT\r\nSUMMARY:Tomorrow\r\nDTSTART:{}\r\nDTEND:{}\r\nEND:VEVENT\r\n",
            ical_dt(24),
            ical_dt(25)
        ),
        now + 2,
    )
    .await
    .unwrap();

    // Free/busy for today only (0..24h from base).
    let busy = cs.free_busy_query(&cal.id, ms_at(0), ms_at(24)).await.unwrap();
    assert_eq!(busy.len(), 2, "two events today");

    // Free/busy spanning both days.
    let busy = cs.free_busy_query(&cal.id, ms_at(0), ms_at(48)).await.unwrap();
    assert_eq!(busy.len(), 3, "three events across two days");
}

#[tokio::test]
async fn e2e_delete_calendar_cascade() {
    let (cs, _) = store().await;
    let now = BASE_MS;

    let cal = cs.create_calendar("Temp", None, None, None, now).await.unwrap();
    cs.create_event(&cal.id, &format!("BEGIN:VEVENT\r\nSUMMARY:Gone\r\nDTSTART:{}\r\nEND:VEVENT\r\n", ical_dt(0)), now)
        .await
        .unwrap();

    cs.delete_calendar(&cal.id).await.unwrap();

    assert!(cs.get_calendar(&cal.id).await.is_err());
    let cals = cs.list_calendars(None).await.unwrap();
    assert!(cals.is_empty());
}
