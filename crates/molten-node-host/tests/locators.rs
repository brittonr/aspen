#[test]
fn remote_locators_are_denied_in_both_local_path_domains() {
    type Local = molten_node_host::local_store::RelativeLocator;
    type Node = molten_node_host::node_state::RelativePath;
    for value in [
        "iroh:value",
        "http:value",
        "https:value",
        "blake3:value",
        "custom://host",
    ] {
        assert!(Local::parse(value).is_err(), "{value}");
        assert!(Node::parse(value).is_err(), "{value}");
    }
    for value in ["relative/file", "HTTP:value", "ssh:value", "relative/http:value"] {
        assert!(Local::parse(value).is_ok(), "{value}");
        assert!(Node::parse(value).is_ok(), "{value}");
    }
}

#[test]
fn platform_prefixed_and_oversized_paths_are_denied() {
    type Local = molten_node_host::local_store::RelativeLocator;
    type Node = molten_node_host::node_state::RelativePath;
    assert!(Local::parse("").is_err());
    assert!(Node::parse("").is_err());
    for value in [
        "C://host",
        "C:relative",
        "z:relative",
        "\\\\server\\share",
        "nested\\value",
    ] {
        assert!(Local::parse(value).is_err(), "{value}");
        assert!(Node::parse(value).is_err(), "{value}");
    }
    let oversized = format!("http:{}", "x".repeat(4096));
    assert!(Node::parse(&oversized).is_err());
}
