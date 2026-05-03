/// CI request variant names owned by the `ci` optional app.
///
/// CI service executors use this same list for request-handle registration so
/// request metadata and dispatch coverage cannot drift independently.
pub const CI_REQUEST_VARIANTS: &[&str] = &[
    "CiCancelRun",
    "CiGetArtifact",
    "CiGetJobLogs",
    "CiGetJobOutput",
    "CiGetRunReceipt",
    "CiGetStatus",
    "CiGetRefStatus",
    "CiListArtifacts",
    "CiListRuns",
    "CiSubscribeLogs",
    "CiTriggerPipeline",
    "CiUnwatchRepo",
    "CiWatchRepo",
];

pub(super) const REQUIRED_APP_VARIANTS: &[&str] = CI_REQUEST_VARIANTS;

pub(super) fn required_app(variant_name: &str) -> Option<&'static str> {
    if REQUIRED_APP_VARIANTS.contains(&variant_name) {
        Some("ci")
    } else {
        None
    }
}
