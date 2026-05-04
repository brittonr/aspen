use alloc::string::ToString;

use aspen_auth_core::Operation;

use super::super::ClientRpcRequest;

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    to_operation_ci_pipeline(request).or_else(|| to_operation_cache_snix(request))
}

fn to_operation_ci_pipeline(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        ClientRpcRequest::CiGetStatus { run_id } | ClientRpcRequest::CiGetRunReceipt { run_id } => {
            Some(Some(Operation::Read {
                key: format!("_ci:runs:{}", run_id),
            }))
        }
        ClientRpcRequest::CiGetRefStatus { repo_id, ref_name } => Some(Some(Operation::Read {
            key: format!("_ci:ref-status:{}:{}", repo_id, ref_name),
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
        ClientRpcRequest::NixCacheGetPublicKey => Some(Some(Operation::Read {
            key: "_sys:nix-cache:public-key".to_string(),
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

        ClientRpcRequest::SnixDirectoryGet { digest } => Some(Some(Operation::SnixRead {
            resource: format!("dir:{digest}"),
        })),
        ClientRpcRequest::SnixDirectoryPut { .. } => Some(Some(Operation::SnixWrite {
            resource: "dir:".to_string(),
        })),
        ClientRpcRequest::SnixPathInfoGet { digest } => Some(Some(Operation::SnixRead {
            resource: format!("pathinfo:{digest}"),
        })),
        ClientRpcRequest::SnixPathInfoPut { .. } => Some(Some(Operation::SnixWrite {
            resource: "pathinfo:".to_string(),
        })),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use aspen_auth_core::Capability;

    use super::*;

    fn operation_for(request: &ClientRpcRequest) -> Operation {
        to_operation(request).flatten().expect("request should require auth")
    }

    #[test]
    fn snix_requests_require_snix_scoped_operations() {
        let directory_get = operation_for(&ClientRpcRequest::SnixDirectoryGet {
            digest: "dir-digest".to_string(),
        });
        assert!(matches!(directory_get, Operation::SnixRead { resource } if resource == "dir:dir-digest"));

        let directory_put = operation_for(&ClientRpcRequest::SnixDirectoryPut {
            directory_bytes: "encoded-directory".to_string(),
        });
        assert!(matches!(directory_put, Operation::SnixWrite { resource } if resource == "dir:"));

        let pathinfo_get = operation_for(&ClientRpcRequest::SnixPathInfoGet {
            digest: "path-digest".to_string(),
        });
        assert!(matches!(pathinfo_get, Operation::SnixRead { resource } if resource == "pathinfo:path-digest"));

        let pathinfo_put = operation_for(&ClientRpcRequest::SnixPathInfoPut {
            pathinfo_bytes: "encoded-pathinfo".to_string(),
        });
        assert!(matches!(pathinfo_put, Operation::SnixWrite { resource } if resource == "pathinfo:"));
    }

    #[test]
    fn generic_kv_scopes_do_not_authorize_snix_put_operations() {
        let generic_full = Capability::Full {
            prefix: "snix:".to_string(),
        };
        let generic_write = Capability::Write {
            prefix: "snix:".to_string(),
        };
        let snix_write = Capability::SnixWrite {
            resource_prefix: "dir:".to_string(),
        };

        let operation = operation_for(&ClientRpcRequest::SnixDirectoryPut {
            directory_bytes: "encoded-directory".to_string(),
        });

        assert!(!generic_full.authorizes(&operation));
        assert!(!generic_write.authorizes(&operation));
        assert!(snix_write.authorizes(&operation));
    }
}
