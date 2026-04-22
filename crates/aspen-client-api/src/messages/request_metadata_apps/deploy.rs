pub(super) const REQUIRED_APP_VARIANTS: &[&str] = &[
    "ClusterDeploy",
    "ClusterDeployStatus",
    "ClusterRollback",
    "NodeRollback",
    "NodeUpgrade",
];

pub(super) fn required_app(variant_name: &str) -> Option<&'static str> {
    if REQUIRED_APP_VARIANTS.contains(&variant_name) {
        Some("deploy")
    } else {
        None
    }
}
