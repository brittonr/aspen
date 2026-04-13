pub(super) fn required_app(variant_name: &str) -> Option<&'static str> {
    match variant_name {
        "JobCancel" | "JobGet" | "JobList" | "JobQueueStats" | "JobSubmit" | "JobUpdateProgress"
        | "WorkerCompleteJob" | "WorkerDeregister" | "WorkerHeartbeat" | "WorkerPollJobs" | "WorkerRegister"
        | "WorkerStatus" => Some("jobs"),
        _ => None,
    }
}
