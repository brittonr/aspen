use aspen_auth::Operation;

use super::super::ClientRpcRequest;

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        // Calendar CRUD
        ClientRpcRequest::CalendarCreate { .. } => Some(Some(Operation::Write {
            key: "calendar:cal:".to_string(),
            value: vec![],
        })),
        ClientRpcRequest::CalendarDelete { calendar_id } => Some(Some(Operation::Write {
            key: format!("calendar:cal:{}", calendar_id),
            value: vec![],
        })),
        ClientRpcRequest::CalendarList { .. } => Some(Some(Operation::Read {
            key: "calendar:cal:".to_string(),
        })),

        // Event CRUD
        ClientRpcRequest::CalendarCreateEvent { calendar_id, .. } => Some(Some(Operation::Write {
            key: format!("calendar:event:{}:", calendar_id),
            value: vec![],
        })),
        ClientRpcRequest::CalendarGetEvent { event_id } => Some(Some(Operation::Read {
            key: format!("calendar:event:{}", event_id),
        })),
        ClientRpcRequest::CalendarUpdateEvent { event_id, .. } => Some(Some(Operation::Write {
            key: format!("calendar:event:{}", event_id),
            value: vec![],
        })),
        ClientRpcRequest::CalendarDeleteEvent { event_id } => Some(Some(Operation::Write {
            key: format!("calendar:event:{}", event_id),
            value: vec![],
        })),
        ClientRpcRequest::CalendarListEvents { calendar_id, .. } => Some(Some(Operation::Read {
            key: format!("calendar:event:{}:", calendar_id),
        })),
        ClientRpcRequest::CalendarSearchEvents { .. } => Some(Some(Operation::Read {
            key: "calendar:event:".to_string(),
        })),

        // Import/export
        ClientRpcRequest::CalendarImportIcal { calendar_id, .. } => Some(Some(Operation::Write {
            key: format!("calendar:event:{}:", calendar_id),
            value: vec![],
        })),
        ClientRpcRequest::CalendarExportIcal { calendar_id } => Some(Some(Operation::Read {
            key: format!("calendar:event:{}:", calendar_id),
        })),

        // Queries
        ClientRpcRequest::CalendarFreeBusy { calendar_id, .. } => Some(Some(Operation::Read {
            key: format!("calendar:event:{}:", calendar_id),
        })),
        ClientRpcRequest::CalendarExpandRecurrence { event_id, .. } => Some(Some(Operation::Read {
            key: format!("calendar:event:{}", event_id),
        })),

        _ => None,
    }
}
