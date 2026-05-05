use alloc::format;
use alloc::string::String;
use alloc::string::ToString;

use aspen_auth_core::Operation;

use super::super::ClientRpcRequest;

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    to_operation_ci_pipeline(request).or_else(|| to_operation_cache_snix(request))
}

fn to_operation_ci_pipeline(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        ClientRpcRequest::CiGetStatus { run_id } | ClientRpcRequest::CiGetRunReceipt { run_id } => {
            Some(Some(Operation::CiRead {
                resource: format!("run:{run_id}"),
            }))
        }
        ClientRpcRequest::CiGetRefStatus { repo_id, ref_name } => Some(Some(Operation::CiRead {
            resource: format!("repo:{repo_id}:ref:{ref_name}"),
        })),
        ClientRpcRequest::CiListRuns { repo_id, .. } => Some(Some(Operation::CiRead {
            resource: ci_repo_resource(repo_id.as_deref()),
        })),
        ClientRpcRequest::CiTriggerPipeline { repo_id, .. }
        | ClientRpcRequest::CiWatchRepo { repo_id }
        | ClientRpcRequest::CiUnwatchRepo { repo_id } => Some(Some(Operation::CiWrite {
            resource: format!("repo:{repo_id}"),
        })),
        ClientRpcRequest::CiCancelRun { run_id, .. } => Some(Some(Operation::CiWrite {
            resource: format!("run:{run_id}"),
        })),
        ClientRpcRequest::CiListArtifacts { job_id, run_id } => Some(Some(Operation::CiRead {
            resource: match run_id {
                Some(run_id) => format!("run:{run_id}:artifact:{job_id}"),
                None => format!("artifact:{job_id}"),
            },
        })),
        ClientRpcRequest::CiGetArtifact { blob_hash } => Some(Some(Operation::CiRead {
            resource: format!("artifact:{blob_hash}"),
        })),
        ClientRpcRequest::CiGetJobLogs { run_id, job_id, .. }
        | ClientRpcRequest::CiSubscribeLogs { run_id, job_id, .. } => Some(Some(Operation::CiRead {
            resource: format!("run:{run_id}:log:{job_id}"),
        })),
        ClientRpcRequest::CiGetJobOutput { run_id, job_id } => Some(Some(Operation::CiRead {
            resource: format!("run:{run_id}:output:{job_id}"),
        })),

        _ => None,
    }
}

fn ci_repo_resource(repo_id: Option<&str>) -> String {
    match repo_id {
        Some(repo_id) => format!("repo:{repo_id}"),
        None => "run:".to_string(),
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
    fn ci_requests_require_ci_scoped_operations() {
        let status = operation_for(&ClientRpcRequest::CiGetStatus {
            run_id: "run-1".to_string(),
        });
        assert!(matches!(status, Operation::CiRead { resource } if resource == "run:run-1"));

        let trigger = operation_for(&ClientRpcRequest::CiTriggerPipeline {
            repo_id: "repo-1".to_string(),
            ref_name: "refs/heads/main".to_string(),
            commit_hash: None,
        });
        assert!(matches!(trigger, Operation::CiWrite { resource } if resource == "repo:repo-1"));

        let logs = operation_for(&ClientRpcRequest::CiGetJobLogs {
            run_id: "run-1".to_string(),
            job_id: "job-1".to_string(),
            start_index: 0,
            limit: None,
        });
        assert!(matches!(logs, Operation::CiRead { resource } if resource == "run:run-1:log:job-1"));
    }

    #[test]
    fn generic_ci_prefixes_do_not_authorize_ci_requests() {
        let generic_read = Capability::Read {
            prefix: "_ci:".to_string(),
        };
        let generic_write = Capability::Write {
            prefix: "_ci:".to_string(),
        };
        let ci_read = Capability::CiRead {
            resource_prefix: "run:".to_string(),
        };
        let ci_write = Capability::CiWrite {
            resource_prefix: "repo:".to_string(),
        };

        let status = operation_for(&ClientRpcRequest::CiGetStatus {
            run_id: "run-1".to_string(),
        });
        assert!(!generic_read.authorizes(&status));
        assert!(ci_read.authorizes(&status));

        let trigger = operation_for(&ClientRpcRequest::CiTriggerPipeline {
            repo_id: "repo-1".to_string(),
            ref_name: "refs/heads/main".to_string(),
            commit_hash: None,
        });
        assert!(!generic_write.authorizes(&trigger));
        assert!(ci_write.authorizes(&trigger));
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
