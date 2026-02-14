use aspen_auth::Operation;

use super::super::ClientRpcRequest;

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        // DNS write operations
        ClientRpcRequest::DnsSetRecord { domain, .. }
        | ClientRpcRequest::DnsDeleteRecord { domain, .. }
        | ClientRpcRequest::DnsSetZone { name: domain, .. }
        | ClientRpcRequest::DnsDeleteZone { name: domain, .. } => Some(Some(Operation::Write {
            key: format!("_dns:{domain}"),
            value: vec![],
        })),

        // DNS read operations
        ClientRpcRequest::DnsGetRecord { domain, .. }
        | ClientRpcRequest::DnsGetRecords { domain }
        | ClientRpcRequest::DnsResolve { domain, .. }
        | ClientRpcRequest::DnsGetZone { name: domain } => Some(Some(Operation::Read {
            key: format!("_dns:{domain}"),
        })),
        ClientRpcRequest::DnsScanRecords { prefix, .. } => Some(Some(Operation::Read {
            key: format!("_dns:{prefix}"),
        })),
        ClientRpcRequest::DnsListZones => Some(Some(Operation::Read {
            key: "_dns:".to_string(),
        })),

        _ => None,
    }
}
