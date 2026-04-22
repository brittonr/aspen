pub(super) const REQUIRED_APP_VARIANTS: &[&str] = &[
    "AutomergeApplyChanges",
    "AutomergeCreate",
    "AutomergeDelete",
    "AutomergeExists",
    "AutomergeGenerateSyncMessage",
    "AutomergeGet",
    "AutomergeGetMetadata",
    "AutomergeList",
    "AutomergeMerge",
    "AutomergeReceiveSyncMessage",
    "AutomergeSave",
];

pub(super) fn required_app(variant_name: &str) -> Option<&'static str> {
    if REQUIRED_APP_VARIANTS.contains(&variant_name) {
        Some("automerge")
    } else {
        None
    }
}
