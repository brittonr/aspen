pub(super) fn required_app(variant_name: &str) -> Option<&'static str> {
    match variant_name {
        "HookGetMetrics" | "HookList" | "HookTrigger" => Some("hooks"),
        _ => None,
    }
}
