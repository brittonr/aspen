#[test]
fn runtime_applications_doc_anchors_host_loading_taxonomy() {
    let doc = include_str!("../docs/runtime-applications.md");

    for required in [
        "## Runtime host-loading taxonomy",
        "NativeBuiltIn",
        "NativeProcess",
        "Wasm",
        "Hyperlight",
        "OciImage` ingestion/lowering",
        "OciLoweringPlan",
        "RuntimeHostKind::OciContainer` remains only as a dev/unsafe marker",
        "rejected as the default production boundary",
        "MicroVm",
        "Unikernel",
        "crates/aspen-runtime-core",
        "not loaded through `dlopen`-style native plugins",
        "opaque handles, hashes, or redacted summaries",
    ] {
        assert!(doc.contains(required), "missing runtime host-loading doc anchor: {required}");
    }
}
