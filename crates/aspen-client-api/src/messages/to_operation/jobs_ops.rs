use alloc::format;
use alloc::string::String;
use alloc::string::ToString;

use aspen_auth_core::Operation;

use super::super::ClientRpcRequest;

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        ClientRpcRequest::JobSubmit { job_type, .. } => Some(Some(Operation::JobsWrite {
            resource: format!("type:{job_type}"),
        })),
        ClientRpcRequest::JobCancel { job_id, .. } | ClientRpcRequest::JobUpdateProgress { job_id, .. } => {
            Some(Some(Operation::JobsWrite {
                resource: format!("job:{job_id}"),
            }))
        }
        ClientRpcRequest::WorkerRegister { worker_id, .. }
        | ClientRpcRequest::WorkerHeartbeat { worker_id, .. }
        | ClientRpcRequest::WorkerDeregister { worker_id } => Some(Some(Operation::JobsWrite {
            resource: format!("worker:{worker_id}"),
        })),

        ClientRpcRequest::JobGet { job_id } => Some(Some(Operation::JobsRead {
            resource: format!("job:{job_id}"),
        })),
        ClientRpcRequest::JobList { job_type, .. } => Some(Some(Operation::JobsRead {
            resource: job_list_resource(job_type.as_deref()),
        })),
        ClientRpcRequest::JobQueueStats => Some(Some(Operation::JobsRead {
            resource: "queue:stats".to_string(),
        })),
        ClientRpcRequest::WorkerStatus => Some(Some(Operation::JobsRead {
            resource: "worker:".to_string(),
        })),

        ClientRpcRequest::WorkerPollJobs { worker_id, .. } => Some(Some(Operation::JobsRead {
            resource: format!("worker:{worker_id}:jobs"),
        })),
        ClientRpcRequest::WorkerCompleteJob { worker_id, .. } => Some(Some(Operation::JobsWrite {
            resource: format!("worker:{worker_id}:complete"),
        })),

        _ => None,
    }
}

fn job_list_resource(job_type: Option<&str>) -> String {
    match job_type {
        Some(job_type) => format!("type:{job_type}"),
        None => "job:".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use aspen_auth_core::Capability;

    use super::*;

    fn operation_for(request: &ClientRpcRequest) -> Operation {
        to_operation(request).flatten().expect("request should require auth")
    }

    #[test]
    fn job_requests_use_jobs_capabilities_not_generic_prefixes() {
        let submit = operation_for(&ClientRpcRequest::JobSubmit {
            job_type: "build".to_string(),
            payload: "{}".to_string(),
            priority: None,
            timeout_ms: None,
            max_retries: None,
            retry_delay_ms: None,
            schedule: None,
            tags: vec![],
        });
        assert!(matches!(submit, Operation::JobsWrite { resource } if resource == "type:build"));

        let get = operation_for(&ClientRpcRequest::JobGet {
            job_id: "job-1".to_string(),
        });
        assert!(matches!(get, Operation::JobsRead { resource } if resource == "job:job-1"));

        let poll = operation_for(&ClientRpcRequest::WorkerPollJobs {
            worker_id: "worker-a".to_string(),
            job_types: vec!["build".to_string()],
            max_jobs: 1,
            visibility_timeout_secs: 30,
        });
        assert!(matches!(poll, Operation::JobsRead { resource } if resource == "worker:worker-a:jobs"));
    }

    #[test]
    fn generic_jobs_prefixes_do_not_authorize_job_requests() {
        let generic_read = Capability::Read {
            prefix: "_jobs:".to_string(),
        };
        let generic_write = Capability::Write {
            prefix: "_jobs:".to_string(),
        };
        let jobs_read = Capability::JobsRead {
            resource_prefix: "job:".to_string(),
        };
        let jobs_write = Capability::JobsWrite {
            resource_prefix: "type:".to_string(),
        };

        let get = operation_for(&ClientRpcRequest::JobGet {
            job_id: "job-1".to_string(),
        });
        assert!(!generic_read.authorizes(&get));
        assert!(jobs_read.authorizes(&get));

        let submit = operation_for(&ClientRpcRequest::JobSubmit {
            job_type: "build".to_string(),
            payload: "{}".to_string(),
            priority: None,
            timeout_ms: None,
            max_retries: None,
            retry_delay_ms: None,
            schedule: None,
            tags: vec![],
        });
        assert!(!generic_write.authorizes(&submit));
        assert!(jobs_write.authorizes(&submit));
    }
}
