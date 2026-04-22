pub(super) const REQUIRED_APP_VARIANTS: &[&str] = &[
    "ExecuteSql",
];

pub(super) fn required_app(variant_name: &str) -> Option<&'static str> {
    if REQUIRED_APP_VARIANTS.contains(&variant_name) {
        Some("sql")
    } else {
        None
    }
}
