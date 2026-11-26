//! Regression tests for opening an existing node-state root.

const MARKER: &str = "root-marker";

/// A test-owned directory under the system temporary directory; only this exact path is
/// removed.
struct OwnedTemp(std::path::PathBuf);

impl OwnedTemp {
    fn new(label: &str) -> Self {
        static SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("molten-node-host-{label}-{}-{sequence}", std::process::id()));
        std::fs::create_dir(&path).expect("test-owned temporary directory");
        Self(path)
    }
}

impl Drop for OwnedTemp {
    fn drop(&mut self) {
        let _removed = std::fs::remove_dir_all(&self.0);
    }
}

// r[verify molten.runtime_spine.admission_and_capability_hardening]
#[test]
fn a_root_swapped_for_a_symlink_after_the_check_is_never_followed() {
    let temp = OwnedTemp::new("root-swap");
    let real = temp.0.join("state");
    let evil = temp.0.join("evil");
    std::fs::create_dir(&real).expect("real root");
    std::fs::write(real.join(MARKER), b"real").expect("real marker");
    std::fs::create_dir(&evil).expect("evil target");
    std::fs::write(evil.join(MARKER), b"evil").expect("evil marker");

    let opened = super::DirectoryRoot::open(&real).expect("positive control");
    assert_eq!(opened.inner.dir.read(MARKER).expect("marker"), b"real");

    // The check sees the real directory; the root is then replaced by a symlink before the open.
    let (swap_real, swap_evil) = (real.clone(), evil.clone());
    super::open_seam::set_after_bootstrap_check(move || {
        std::fs::rename(&swap_real, swap_real.with_extension("parked")).expect("park real root");
        std::os::unix::fs::symlink(&swap_evil, &swap_real).expect("swap in symlink");
    });
    let refused = super::DirectoryRoot::open_existing(&real).expect_err("a swapped root must deny");
    assert!(refused.to_string().contains("must not be a symlink"), "{refused}");
}
