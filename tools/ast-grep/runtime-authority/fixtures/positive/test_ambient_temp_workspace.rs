fn predictable_workspace(label: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("molten-{label}-{}", std::process::id()));
    if root.exists() {
        std::fs::remove_dir_all(&root).expect("remove predictable root");
    }
    std::fs::create_dir_all(&root).expect("create predictable root");
    root
}

fn broad_stale_prefix_cleanup() {
    for entry in std::fs::read_dir(std::env::temp_dir()).expect("scan ambient temporary root") {
        let entry = entry.expect("ambient entry");
        if entry.file_name().to_string_lossy().starts_with("molten-") {
            std::fs::remove_dir_all(entry.path()).expect("delete matching ambient entry");
        }
    }
}
