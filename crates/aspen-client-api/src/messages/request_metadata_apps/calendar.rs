pub(super) const REQUIRED_APP_VARIANTS: &[&str] = &[
    "CalendarCreate",
    "CalendarCreateEvent",
    "CalendarDelete",
    "CalendarDeleteEvent",
    "CalendarExpandRecurrence",
    "CalendarExportIcal",
    "CalendarFreeBusy",
    "CalendarGetEvent",
    "CalendarImportIcal",
    "CalendarList",
    "CalendarListEvents",
    "CalendarSearchEvents",
    "CalendarUpdateEvent",
];

pub(super) fn required_app(variant_name: &str) -> Option<&'static str> {
    if REQUIRED_APP_VARIANTS.contains(&variant_name) {
        Some("calendar")
    } else {
        None
    }
}
