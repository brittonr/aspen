use aspen_auth::Operation;

use super::super::ClientRpcRequest;

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        // Trace writes
        ClientRpcRequest::TraceIngest { .. } => Some(Some(Operation::Write {
            key: "_sys:traces:".to_string(),
            value: vec![],
        })),
        // Trace reads
        ClientRpcRequest::TraceList { .. }
        | ClientRpcRequest::TraceGet { .. }
        | ClientRpcRequest::TraceSearch { .. } => Some(Some(Operation::Read {
            key: "_sys:traces:".to_string(),
        })),
        // Metric writes
        ClientRpcRequest::MetricIngest { .. } => Some(Some(Operation::Write {
            key: "_sys:metrics:".to_string(),
            value: vec![],
        })),
        // Metric reads
        ClientRpcRequest::MetricList { .. } | ClientRpcRequest::MetricQuery { .. } => Some(Some(Operation::Read {
            key: "_sys:metrics:".to_string(),
        })),
        // Alert writes
        ClientRpcRequest::AlertCreate { .. } | ClientRpcRequest::AlertDelete { .. } => Some(Some(Operation::Write {
            key: "_sys:alerts:rule:".to_string(),
            value: vec![],
        })),
        // Alert evaluate (reads metrics, writes state)
        ClientRpcRequest::AlertEvaluate { .. } => Some(Some(Operation::Write {
            key: "_sys:alerts:state:".to_string(),
            value: vec![],
        })),
        // Alert reads
        ClientRpcRequest::AlertList | ClientRpcRequest::AlertGet { .. } => Some(Some(Operation::Read {
            key: "_sys:alerts:".to_string(),
        })),
        _ => None,
    }
}
