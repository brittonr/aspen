pub(super) const REQUIRED_APP_VARIANTS: &[&str] = &[
    "JobCancel",
    "JobGet",
    "JobList",
    "JobQueueStats",
    "JobSubmit",
    "JobUpdateProgress",
    "WorkerCompleteJob",
    "WorkerDeregister",
    "WorkerHeartbeat",
    "WorkerPollJobs",
    "WorkerRegister",
    "WorkerStatus",
];

pub(super) fn required_app(variant_name: &str) -> Option<&'static str> {
    if REQUIRED_APP_VARIANTS.contains(&variant_name) {
        Some("jobs")
    } else {
        None
    }
}
