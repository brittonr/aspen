use aspen_auth::Operation;

use super::super::ClientRpcRequest;

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    to_operation_ci_pipeline(request).or_else(|| to_operation_cache_snix(request))
}

fn to_operation_ci_pipeline(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        ClientRpcRequest::CiGetStatus { run_id } => Some(Some(Operation::Read {
            key: format!("_ci:runs:{}", run_id),
        })),
        ClientRpcRequest::CiListRuns { repo_id, .. } => Some(Some(Operation::Read {
            key: format!("_ci:runs:{}", repo_id.as_deref().unwrap_or("")),
        })),
        ClientRpcRequest::CiTriggerPipeline { repo_id, .. }
        | ClientRpcRequest::CiWatchRepo { repo_id }
        | ClientRpcRequest::CiUnwatchRepo { repo_id } => Some(Some(Operation::Write {
            key: format!("_ci:repos:{}", repo_id),
            value: vec![],
        })),
        ClientRpcRequest::CiCancelRun { run_id, .. } => Some(Some(Operation::Write {
            key: format!("_ci:runs:{}", run_id),
            value: vec![],
        })),
        ClientRpcRequest::CiListArtifacts { job_id, run_id } => Some(Some(Operation::Read {
            key: format!("_ci:artifacts:{}:{}", job_id, run_id.as_deref().unwrap_or("")),
        })),
        ClientRpcRequest::CiGetArtifact { blob_hash } => Some(Some(Operation::Read {
            key: format!("_ci:artifacts:{}", blob_hash),
        })),
        ClientRpcRequest::CiGetJobLogs { run_id, job_id, .. } => Some(Some(Operation::Read {
            key: format!("_ci:logs:{}:{}", run_id, job_id),
        })),
        ClientRpcRequest::CiSubscribeLogs { run_id, job_id, .. } => Some(Some(Operation::Read {
            key: format!("_ci:logs:{}:{}", run_id, job_id),
        })),
        ClientRpcRequest::CiGetJobOutput { run_id, job_id } => Some(Some(Operation::Read {
            key: format!("_ci:runs:{}:{}", run_id, job_id),
        })),

        _ => None,
    }
}

fn to_operation_cache_snix(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        ClientRpcRequest::CacheQuery { store_hash } | ClientRpcRequest::CacheDownload { store_hash } => {
            Some(Some(Operation::Read {
                key: format!("_cache:narinfo:{store_hash}"),
            }))
        }
        ClientRpcRequest::CacheStats => Some(Some(Operation::Read {
            key: "_cache:stats".to_string(),
        })),

        #[cfg(feature = "ci")]
        ClientRpcRequest::CacheMigrationStart { .. } | ClientRpcRequest::CacheMigrationCancel => {
            Some(Some(Operation::ClusterAdmin {
                action: "cache_migration".to_string(),
            }))
        }
        #[cfg(feature = "ci")]
        ClientRpcRequest::CacheMigrationStatus | ClientRpcRequest::CacheMigrationValidate { .. } => {
            Some(Some(Operation::Read {
                key: "_cache:migration:".to_string(),
            }))
        }

        ClientRpcRequest::SnixDirectoryGet { digest } => Some(Some(Operation::Read {
            key: format!("snix:dir:{digest}"),
        })),
        ClientRpcRequest::SnixDirectoryPut { .. } => Some(Some(Operation::Write {
            key: "snix:dir:".to_string(),
            value: vec![],
        })),
        ClientRpcRequest::SnixPathInfoGet { digest } => Some(Some(Operation::Read {
            key: format!("snix:pathinfo:{digest}"),
        })),
        ClientRpcRequest::SnixPathInfoPut { .. } => Some(Some(Operation::Write {
            key: "snix:pathinfo:".to_string(),
            value: vec![],
        })),

        _ => None,
    }
}
