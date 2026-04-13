pub(super) fn required_app(variant_name: &str) -> Option<&'static str> {
    match variant_name {
        "ExecuteSql" => Some("sql"),
        _ => None,
    }
}
