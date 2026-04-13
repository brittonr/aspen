pub(super) fn required_app(variant_name: &str) -> Option<&'static str> {
    match variant_name {
        "CalendarCreate"
        | "CalendarCreateEvent"
        | "CalendarDelete"
        | "CalendarDeleteEvent"
        | "CalendarExpandRecurrence"
        | "CalendarExportIcal"
        | "CalendarFreeBusy"
        | "CalendarGetEvent"
        | "CalendarImportIcal"
        | "CalendarList"
        | "CalendarListEvents"
        | "CalendarSearchEvents"
        | "CalendarUpdateEvent" => Some("calendar"),
        _ => None,
    }
}
