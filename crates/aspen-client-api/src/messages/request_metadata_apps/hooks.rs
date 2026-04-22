pub(super) const REQUIRED_APP_VARIANTS: &[&str] = &[
    "HookGetMetrics",
    "HookList",
    "HookTrigger",
];

pub(super) fn required_app(variant_name: &str) -> Option<&'static str> {
    if REQUIRED_APP_VARIANTS.contains(&variant_name) {
        Some("hooks")
    } else {
        None
    }
}
