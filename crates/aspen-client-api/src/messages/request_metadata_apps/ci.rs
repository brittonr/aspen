pub(super) const REQUIRED_APP_VARIANTS: &[&str] = &[
    "CiCancelRun",
    "CiGetArtifact",
    "CiGetJobLogs",
    "CiGetJobOutput",
    "CiGetStatus",
    "CiGetRefStatus",
    "CiListArtifacts",
    "CiListRuns",
    "CiSubscribeLogs",
    "CiTriggerPipeline",
    "CiUnwatchRepo",
    "CiWatchRepo",
];

pub(super) fn required_app(variant_name: &str) -> Option<&'static str> {
    if REQUIRED_APP_VARIANTS.contains(&variant_name) {
        Some("ci")
    } else {
        None
    }
}
