#[test]
fn shared_recognition_preserves_boundary_specific_errors() {
    type Local = molten_node_host::local_store::LocalStorePath;
    type Node = molten_node_host::node_state::NodeStatePath;
    type Error = molten_node_host::error::MoltenError;
    for value in [
        "iroh:value",
        "http:value",
        "https:value",
        "blake3:value",
        "custom://host",
    ] {
        assert_eq!(
            Local::parse(value).unwrap_err(),
            Error::invalid_harness(format!(
                "remote or content locator {value} cannot be used as a local filesystem path"
            ))
        );
        assert_eq!(
            Node::parse(value).unwrap_err(),
            Error::invalid_harness(format!("remote or content locator {value} cannot become node state authority"))
        );
    }
    for value in ["relative/file", "HTTP:value", "ssh:value", "relative/http:value"] {
        assert!(Local::parse(value).is_ok(), "{value}");
        assert!(Node::parse(value).is_ok(), "{value}");
    }
}

#[test]
fn earlier_admission_checks_keep_their_precedence() {
    type Local = molten_node_host::local_store::LocalStorePath;
    type Node = molten_node_host::node_state::NodeStatePath;
    type Error = molten_node_host::error::MoltenError;
    assert_eq!(Local::parse("").unwrap_err(), Error::invalid_harness("local store path cannot be empty"));
    assert_eq!(Node::parse("").unwrap_err(), Error::invalid_harness("node state path cannot be empty"));
    let drive = "C://host";
    assert_eq!(
        Local::parse(drive).unwrap_err(),
        Error::invalid_harness(format!(
            "platform-prefixed local store path {drive} is not portable relative authority"
        ))
    );
    assert_eq!(
        Node::parse(drive).unwrap_err(),
        Error::invalid_harness(format!("platform-prefixed node state path {drive} is not relative authority"))
    );
    let oversized = format!("http:{}", "x".repeat(4096));
    assert_eq!(
        Node::parse(&oversized).unwrap_err(),
        Error::invalid_harness("node state path length 4101 exceeds maximum 4096")
    );
}
