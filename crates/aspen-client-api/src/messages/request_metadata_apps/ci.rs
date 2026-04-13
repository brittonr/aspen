pub(super) fn required_app(variant_name: &str) -> Option<&'static str> {
    match variant_name {
        "CiCancelRun" | "CiGetArtifact" | "CiGetJobLogs" | "CiGetJobOutput" | "CiGetStatus" | "CiGetRefStatus"
        | "CiListArtifacts" | "CiListRuns" | "CiSubscribeLogs" | "CiTriggerPipeline" | "CiUnwatchRepo"
        | "CiWatchRepo" => Some("ci"),
        _ => None,
    }
}
