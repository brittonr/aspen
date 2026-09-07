#[test]
fn enumeration_is_sorted_and_entries_keep_their_authority() {
    let temp = cap_tempfile::TempDir::new(cap_std::ambient_authority()).unwrap();
    let root = molten_node_host::node_state::NodeStateRoot::from_dir(temp.try_clone().unwrap());
    let inbox = root.control_inbox().unwrap();
    for name in ["zeta", "alpha"] {
        let path = molten_node_host::node_state::NodeStatePath::parse(name).unwrap();
        inbox.write(&path, name.as_bytes()).unwrap();
    }
    let entries = inbox.list_entries().unwrap();
    assert_eq!(entries.iter().map(|entry| entry.name.as_str()).collect::<Vec<_>>(), ["alpha", "zeta"]);
    let entry = &entries[0];
    assert_eq!(root.clone().control_inbox().unwrap().read_entry(entry, 5).unwrap(), b"alpha");

    let separate_root = molten_node_host::node_state::NodeStateRoot::from_dir(temp.try_clone().unwrap());
    let separate_inbox = separate_root.control_inbox().unwrap();
    let outbox = root.control_outbox().unwrap();
    let subdir = inbox.open_subdir(&molten_node_host::node_state::NodeStatePath::parse("nested").unwrap()).unwrap();
    for view in [&separate_inbox, &outbox, &subdir] {
        view.write(&entry.path, b"other").unwrap();
        assert!(view.read_entry(entry, 5).is_err());
        assert!(view.remove_entry(entry).is_err());
        assert_eq!(view.read(&entry.path, 5).unwrap(), b"other");
    }
    // A separately acquired root addresses the same file but does not share entry authority.
    assert_eq!(inbox.read_entry(entry, 5).unwrap(), b"other");
    inbox.remove_entry(entry).unwrap();
    assert!(!inbox.try_exists(&entry.path).unwrap());
    assert_eq!(outbox.read(&entry.path, 5).unwrap(), b"other");
}

#[test]
fn locators_keep_normalization_and_bounds_without_io() {
    for value in [
        "",
        ".",
        "../escape",
        "/absolute",
        "C:relative",
        "a\\b",
        "http://host",
        "iroh:peer",
        "blake3:value",
    ] {
        assert!(molten_node_host::node_state::NodeStatePath::parse(value).is_err(), "{value}");
    }
    let normalized = molten_node_host::node_state::NodeStatePath::parse("a/./b").unwrap();
    assert_eq!(normalized.as_path(), std::path::Path::new("a/b"));
    assert!(normalized.join_segment("x/y").is_err());
    assert_eq!(normalized.join_segment("x").unwrap().as_path(), std::path::Path::new("a/b/x"));
    let maximum = molten_node_host::node_state::NodeStatePath::parse(&["a"; 32].join("/")).unwrap();
    assert!(maximum.join("a").is_err());
    assert!(molten_node_host::node_state::NodeStatePath::parse(&["a"; 33].join("/")).is_err());
    assert!(molten_node_host::node_state::NodeStatePath::parse(&"a".repeat(4096)).is_ok());
    assert!(molten_node_host::node_state::NodeStatePath::parse(&"a".repeat(4097)).is_err());
}

#[cfg(unix)]
#[test]
fn enumeration_rejects_non_utf8_names_without_removing_them() {
    use std::os::unix::ffi::OsStringExt;
    let temp = cap_tempfile::TempDir::new(cap_std::ambient_authority()).unwrap();
    let name = std::ffi::OsString::from_vec(vec![0xff]);
    let root = molten_node_host::node_state::NodeStateRoot::from_dir(temp.try_clone().unwrap());
    let inbox = root.control_inbox().unwrap();
    let dir = temp.open_dir("control/inbox").unwrap();
    dir.write(&name, b"preserved").unwrap();
    assert!(inbox.list_entries().is_err());
    assert_eq!(dir.read(&name).unwrap(), b"preserved");
}
