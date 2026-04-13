pub(super) fn required_app(variant_name: &str) -> Option<&'static str> {
    match variant_name {
        "AutomergeApplyChanges"
        | "AutomergeCreate"
        | "AutomergeDelete"
        | "AutomergeExists"
        | "AutomergeGenerateSyncMessage"
        | "AutomergeGet"
        | "AutomergeGetMetadata"
        | "AutomergeList"
        | "AutomergeMerge"
        | "AutomergeReceiveSyncMessage"
        | "AutomergeSave" => Some("automerge"),
        _ => None,
    }
}
