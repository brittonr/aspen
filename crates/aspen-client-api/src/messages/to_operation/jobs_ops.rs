use aspen_auth::Operation;

use super::super::ClientRpcRequest;

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        // Job write operations
        ClientRpcRequest::JobSubmit { .. }
        | ClientRpcRequest::JobCancel { .. }
        | ClientRpcRequest::JobUpdateProgress { .. }
        | ClientRpcRequest::WorkerRegister { .. }
        | ClientRpcRequest::WorkerHeartbeat { .. }
        | ClientRpcRequest::WorkerDeregister { .. } => Some(Some(Operation::Write {
            key: "_jobs:".to_string(),
            value: vec![],
        })),

        // Job read operations
        ClientRpcRequest::JobGet { .. }
        | ClientRpcRequest::JobList { .. }
        | ClientRpcRequest::JobQueueStats
        | ClientRpcRequest::WorkerStatus => Some(Some(Operation::Read {
            key: "_jobs:".to_string(),
        })),

        // Worker job coordination operations
        ClientRpcRequest::WorkerPollJobs { worker_id, .. } => Some(Some(Operation::Read {
            key: format!("__worker:{worker_id}:jobs"),
        })),
        ClientRpcRequest::WorkerCompleteJob { worker_id, .. } => Some(Some(Operation::Write {
            key: format!("__worker:{worker_id}:complete"),
            value: vec![],
        })),

        _ => None,
    }
}
