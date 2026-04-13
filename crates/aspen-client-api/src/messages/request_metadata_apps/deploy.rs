pub(super) fn required_app(variant_name: &str) -> Option<&'static str> {
    match variant_name {
        "ClusterDeploy" | "ClusterDeployStatus" | "ClusterRollback" | "NodeRollback" | "NodeUpgrade" => Some("deploy"),
        _ => None,
    }
}
