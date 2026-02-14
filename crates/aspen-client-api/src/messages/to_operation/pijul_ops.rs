use aspen_auth::Operation;

use super::super::ClientRpcRequest;

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        // Pijul write operations
        #[cfg(feature = "pijul")]
        ClientRpcRequest::PijulRepoInit { .. }
        | ClientRpcRequest::PijulChannelCreate { .. }
        | ClientRpcRequest::PijulChannelDelete { .. }
        | ClientRpcRequest::PijulChannelFork { .. }
        | ClientRpcRequest::PijulRecord { .. }
        | ClientRpcRequest::PijulApply { .. }
        | ClientRpcRequest::PijulUnrecord { .. } => Some(Some(Operation::Write {
            key: "_pijul:".to_string(),
            value: vec![],
        })),

        // Pijul read operations
        #[cfg(feature = "pijul")]
        ClientRpcRequest::PijulRepoList { .. }
        | ClientRpcRequest::PijulRepoInfo { .. }
        | ClientRpcRequest::PijulChannelList { .. }
        | ClientRpcRequest::PijulChannelInfo { .. }
        | ClientRpcRequest::PijulLog { .. }
        | ClientRpcRequest::PijulCheckout { .. }
        | ClientRpcRequest::PijulShow { .. }
        | ClientRpcRequest::PijulBlame { .. } => Some(Some(Operation::Read {
            key: "_pijul:".to_string(),
        })),

        _ => None,
    }
}
