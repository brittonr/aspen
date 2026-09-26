use molten_node_host::error::Result;

#[test]
fn enumeration_is_sorted_and_entries_keep_their_authority() -> Result<()> {
    let temp = cap_tempfile::TempDir::new(cap_std::ambient_authority())?;
    let root = molten_node_host::node_state::Root::from_dir(temp.try_clone()?);
    let inbox = root.control_inbox()?;
    for name in ["zeta", "alpha"] {
        let path = molten_node_host::node_state::RelativePath::parse(name)?;
        inbox.write(&path, name.as_bytes())?;
    }
    let entries = inbox.list_entries()?;
    assert_eq!(entries.iter().map(|entry| entry.name.as_str()).collect::<Vec<_>>(), ["alpha", "zeta"]);
    let entry = &entries[0];
    assert_eq!(root.clone().control_inbox()?.read_entry(entry, 5)?, b"alpha");

    let separate_root = molten_node_host::node_state::Root::from_dir(temp.try_clone()?);
    let separate_inbox = separate_root.control_inbox()?;
    let outbox = root.control_outbox()?;
    let subdir = inbox.open_subdir(&molten_node_host::node_state::RelativePath::parse("nested")?)?;
    for view in [&separate_inbox, &outbox, &subdir] {
        view.write(&entry.path, b"other")?;
        assert!(view.read_entry(entry, 5).is_err());
        assert!(view.remove_entry(entry).is_err());
        assert_eq!(view.read(&entry.path, 5)?, b"other");
    }
    // A separately acquired root addresses the same file but does not share entry authority.
    assert_eq!(inbox.read_entry(entry, 5)?, b"other");
    inbox.remove_entry(entry)?;
    assert!(!inbox.try_exists(&entry.path)?);
    assert_eq!(outbox.read(&entry.path, 5)?, b"other");
    Ok(())
}

#[test]
fn locators_keep_normalization_and_bounds_without_io() -> Result<()> {
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
        assert!(molten_node_host::node_state::RelativePath::parse(value).is_err(), "{value}");
    }
    let normalized = molten_node_host::node_state::RelativePath::parse("a/./b")?;
    assert_eq!(normalized.as_path(), std::path::Path::new("a/b"));
    assert!(normalized.join_segment("x/y").is_err());
    assert_eq!(normalized.join_segment("x")?.as_path(), std::path::Path::new("a/b/x"));
    let maximum = molten_node_host::node_state::RelativePath::parse(&["a"; 32].join("/"))?;
    assert!(maximum.join("a").is_err());
    assert!(molten_node_host::node_state::RelativePath::parse(&["a"; 33].join("/")).is_err());
    assert!(molten_node_host::node_state::RelativePath::parse(&"a".repeat(4096)).is_ok());
    assert!(molten_node_host::node_state::RelativePath::parse(&"a".repeat(4097)).is_err());
    Ok(())
}

#[cfg(unix)]
#[test]
fn enumeration_rejects_non_utf8_names_without_removing_them() -> Result<()> {
    use std::os::unix::ffi::OsStringExt;
    let temp = cap_tempfile::TempDir::new(cap_std::ambient_authority())?;
    let name = std::ffi::OsString::from_vec(vec![0xff]);
    let root = molten_node_host::node_state::Root::from_dir(temp.try_clone()?);
    let inbox = root.control_inbox()?;
    let dir = temp.open_dir("control/inbox")?;
    dir.write(&name, b"preserved")?;
    assert!(inbox.list_entries().is_err());
    assert_eq!(dir.read(&name)?, b"preserved");
    Ok(())
}
