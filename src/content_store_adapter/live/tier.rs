use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::path::PathBuf;

use crate::error::Failure;
use crate::error::Result;

const MAX_TIER_ENTRIES: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TierPut {
    Cached(u64),
    RemoteOnly(u64),
}

/// Bounded local content tier. Capacity is a recovery bound: a put that would
/// exceed it is not cached and does not evict another entry.
pub struct LocalTier {
    root: crate::node_state::Scope,
    max_bytes: u64,
    used_bytes: u64,
    sizes: std::collections::BTreeMap<String, u64>,
}

impl LocalTier {
    /// Bootstrap one exclusive tier authority. Descendant operations retain it.
    pub fn open(root: &Path, max_bytes: u64, require_non_rotational: bool) -> Result<Self> {
        if max_bytes == 0 {
            return Err(Failure::invalid_harness("local tier capacity must be nonzero"));
        }
        let scope = crate::node_state::Scope::open(crate::node_state::NamespaceKind::Storage, root)?;
        if require_non_rotational && non_rotational(root) != Some(true) {
            return Err(Failure::invalid_harness("local tier non-rotational backing cannot be established"));
        }
        Self::from_scope(scope, max_bytes)
    }

    pub fn from_recovery_scope(
        scope: crate::node_state::Scope,
        budget: molten_core::content_store_adapter::ContentTierRecoveryBudget,
    ) -> Result<Self> {
        budget
            .validate()
            .map_err(|issue| Failure::invalid_harness(format!("local tier recovery denied: {issue:?}")))?;
        Self::from_scope(scope, budget.max_local_tier_bytes)
    }

    pub fn from_scope(root: crate::node_state::Scope, max_bytes: u64) -> Result<Self> {
        if max_bytes == 0 {
            return Err(Failure::invalid_harness("local tier capacity must be nonzero"));
        }
        let mut sizes = std::collections::BTreeMap::new();
        let mut used_bytes = 0_u64;
        for entry in root.list_entries()? {
            if entry.kind != crate::node_state::EntryKind::RegularFile || !entry.name.ends_with(".chunk") {
                return Err(Failure::invalid_harness("local tier contains an unsupported entry"));
            }
            let hex =
                entry.name.strip_suffix(".chunk").ok_or_else(|| Failure::invalid_harness("invalid cache entry"))?;
            validate_tier_ref(&format!("blake3:{hex}"))?;
            let crate::node_state::FileObservation::Regular(file) = root.observe_file(&entry.path)? else {
                return Err(Failure::invalid_harness("local tier entry changed during admission"));
            };
            used_bytes = used_bytes
                .checked_add(file.size())
                .ok_or_else(|| Failure::invalid_harness("local tier size overflow"))?;
            sizes.insert(entry.name, file.size());
            if sizes.len() > MAX_TIER_ENTRIES || used_bytes > max_bytes {
                return Err(Failure::invalid_harness("local tier exceeds its configured bound"));
            }
        }
        Ok(Self {
            root,
            max_bytes,
            used_bytes,
            sizes,
        })
    }

    pub fn get(&self, object_ref: &str) -> Result<Option<Vec<u8>>> {
        self.get_bounded(object_ref, self.max_bytes)
    }

    pub fn get_bounded(&self, object_ref: &str, max_bytes: u64) -> Result<Option<Vec<u8>>> {
        let max_bytes = max_bytes.min(self.max_bytes).min(crate::node_state::MAX_NODE_STATE_FILE_BYTES);
        let path = Self::entry_path(object_ref)?;
        let file = match self.root.observe_file(&path)? {
            crate::node_state::FileObservation::Missing => return Ok(None),
            crate::node_state::FileObservation::Regular(file) if file.size() <= max_bytes => file,
            _ => return Err(Failure::invalid_harness("local tier leaf is not a bounded regular file")),
        };
        let bytes = file.read_bounded(max_bytes)?;
        if crate::preserves_rail::content_ref_from_bytes(&bytes) != object_ref {
            return Ok(None);
        }
        Ok(Some(bytes))
    }

    pub fn put(&mut self, object_ref: &str, bytes: &[u8]) -> Result<TierPut> {
        if crate::preserves_rail::content_ref_from_bytes(bytes) != object_ref {
            return Err(Failure::invalid_harness("local tier bytes do not match object identity"));
        }
        let path = Self::entry_path(object_ref)?;
        let name = format!("{}.chunk", &object_ref[7..]);
        let length = bytes.len() as u64;
        if length > crate::node_state::MAX_NODE_STATE_FILE_BYTES {
            return Ok(TierPut::RemoteOnly(length));
        }
        let previous = self.sizes.get(&name).copied();
        if previous.is_some() && self.get(object_ref)?.is_some() {
            return Ok(TierPut::Cached(length));
        }
        let next = self
            .used_bytes
            .saturating_sub(previous.unwrap_or(0))
            .checked_add(length)
            .ok_or_else(|| Failure::invalid_harness("local tier size overflow"))?;
        if next > self.max_bytes || (previous.is_none() && self.sizes.len() >= MAX_TIER_ENTRIES) {
            return Ok(TierPut::RemoteOnly(length));
        }
        // Reserve pessimistically before writing: a torn cache write must not
        // free capacity for additional entries until explicit removal/reopen.
        self.used_bytes = next;
        self.sizes.insert(name, length);
        self.root.write(&path, bytes)?;
        Ok(TierPut::Cached(length))
    }

    pub fn remove(&mut self, object_ref: &str) -> Result<()> {
        let path = Self::entry_path(object_ref)?;
        if self.root.try_exists(&path)? {
            self.root.remove_regular_file(&path)?;
        }
        if let Some(size) = self.sizes.remove(&format!("{}.chunk", &object_ref[7..])) {
            self.used_bytes = self.used_bytes.saturating_sub(size);
        }
        Ok(())
    }

    pub fn used_bytes(&self) -> u64 {
        self.used_bytes
    }
    pub fn max_bytes(&self) -> u64 {
        self.max_bytes
    }

    fn entry_path(object_ref: &str) -> Result<crate::node_state::RelativePath> {
        validate_tier_ref(object_ref)?;
        crate::node_state::RelativePath::parse(&format!("{}.chunk", &object_ref[7..]))
    }
}

fn validate_tier_ref(object_ref: &str) -> Result<()> {
    let hex = object_ref
        .strip_prefix("blake3:")
        .ok_or_else(|| Failure::invalid_harness("local tier key is not a BLAKE3 reference"))?;
    if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()) {
        return Err(Failure::invalid_harness("local tier key is not canonical"));
    }
    Ok(())
}

/// Reads the queue rotation flag for the block device backing `path`.
pub fn non_rotational(path: &Path) -> Option<bool> {
    let device = std::fs::metadata(path).ok()?.dev();
    let major = nix_major(device);
    let minor = nix_minor(device);
    let link = PathBuf::from(format!("/sys/dev/block/{major}:{minor}"));
    let mut current = std::fs::canonicalize(link).ok()?;
    loop {
        let rotation = current.join("queue/rotational");
        if let Ok(value) = std::fs::read_to_string(&rotation) {
            return match value.trim() {
                "0" => Some(true),
                "1" => Some(false),
                _ => None,
            };
        }
        current = current.parent()?.to_path_buf();
    }
}

#[cfg(unix)]
fn nix_major(device: u64) -> u64 {
    ((device >> 8) & 0xfff) | ((device >> 32) & !0xfff)
}

#[cfg(unix)]
fn nix_minor(device: u64) -> u64 {
    (device & 0xff) | ((device >> 12) & !0xff)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires MOLTEN_NVME_ROOT on a non-rotational device"]
    fn tier_accepts_a_non_rotational_backing_device() {
        let root = std::env::var("MOLTEN_NVME_ROOT").expect("nvme root");
        let temporary = tempfile::tempdir_in(root).expect("temporary tier");
        let tier = LocalTier::open(temporary.path(), 1024, true).expect("nvme tier");
        assert_eq!(tier.max_bytes, 1024);
    }

    #[test]
    fn tier_respects_capacity_and_rejects_corruption() {
        let temporary = tempfile::tempdir().expect("temporary root");
        let mut tier = LocalTier::open(temporary.path(), 16, false).expect("tier");
        let first = b"eight!!!";
        let first_ref = crate::preserves_rail::content_ref_from_bytes(first);
        assert_eq!(tier.put(&first_ref, first).expect("put"), TierPut::Cached(8));
        assert_eq!(tier.get(&first_ref).expect("get").expect("hit"), first);

        let second = b"another-eight!!!";
        assert_eq!(second.len(), 16);
        let second_ref = crate::preserves_rail::content_ref_from_bytes(second);
        assert_eq!(tier.put(&second_ref, second).expect("put"), TierPut::RemoteOnly(16));
        assert!(tier.get(&second_ref).expect("get").is_none());

        std::fs::write(temporary.path().join(format!("{}.chunk", &first_ref[7..])), b"tampered").expect("tamper");
        assert!(tier.get(&first_ref).expect("get").is_none());
        assert_eq!(tier.put(&first_ref, first).expect("repair"), TierPut::Cached(8));
        assert_eq!(tier.put(&first_ref, first).expect("repeat"), TierPut::Cached(8));
        assert_eq!(tier.used_bytes(), 8);
        drop(tier);
        let tier = LocalTier::open(temporary.path(), 16, false).expect("reopen");
        assert_eq!(tier.used_bytes(), 8);
        assert_eq!(tier.get(&first_ref).expect("get").expect("hit"), first);
    }

    #[test]
    fn retained_root_replacement_does_not_redirect_cache_writes() {
        let directory = tempfile::tempdir().expect("parent");
        let root = directory.path().join("tier");
        let moved = directory.path().join("retained");
        let mut tier = LocalTier::open(&root, 5, false).expect("tier");
        std::fs::rename(&root, &moved).expect("move opened root");
        std::fs::create_dir(&root).expect("replacement");
        let reference = crate::preserves_rail::content_ref_from_bytes(b"hello");
        tier.put(&reference, b"hello").expect("retained authority write");
        let name = format!("{}.chunk", &reference[7..]);
        assert!(moved.join(&name).exists());
        assert!(!root.join(name).exists());
        drop(tier);
        assert!(LocalTier::open(&moved, 4, false).is_err(), "reopen must enforce capacity independently");
    }
    #[test]
    #[ignore = "requires actual MOLTEN_NVME_ROOT, MOLTEN_NVME_SYSFS and MOLTEN_CONTENT_RECEIPT_DIR"]
    fn qualify_actual_nvme_path_capacity_and_reopen() {
        let root = std::env::var("MOLTEN_NVME_ROOT").expect("operator-selected NVMe mount path");
        let expected =
            std::fs::canonicalize(std::env::var("MOLTEN_NVME_SYSFS").expect("expected NVMe block sysfs path"))
                .expect("expected backing device");
        let device = std::fs::metadata(&root).expect("NVMe mount").dev();
        let actual = std::fs::canonicalize(format!("/sys/dev/block/{}:{}", nix_major(device), nix_minor(device)))
            .expect("actual backing block device");
        assert_eq!(actual, expected, "selected mount must resolve to the independently supplied NVMe device");
        assert!(
            actual
                .components()
                .any(|part| part.as_os_str().to_str().is_some_and(|name| name.starts_with("nvme"))),
            "non-rotational alone is not evidence of NVMe"
        );
        assert_eq!(non_rotational(std::path::Path::new(&root)), Some(true));
        let directory = tempfile::tempdir_in(&root).expect("dedicated tier on actual NVMe path");
        let mut tier = LocalTier::open(directory.path(), 5, true).expect("qualified tier");
        let first = crate::preserves_rail::content_ref_from_bytes(b"first");
        let second = crate::preserves_rail::content_ref_from_bytes(b"hello");
        assert_eq!(tier.put(&first, b"first").expect("fill"), TierPut::Cached(5));
        assert_eq!(tier.put(&second, b"hello").expect("cap"), TierPut::RemoteOnly(5));
        drop(tier);
        let tier = LocalTier::open(directory.path(), 5, true).expect("reopen");
        assert_eq!(tier.used_bytes(), 5);
        assert_eq!(tier.get(&first).expect("read").expect("resident"), b"first");
        let value = crate::preserves_rail::record("content-store-nvme-path-qualification-v1", vec![
            crate::preserves_rail::u64_value(nix_major(device)),
            crate::preserves_rail::u64_value(nix_minor(device)),
            crate::preserves_rail::string(crate::preserves_rail::content_ref_from_bytes(
                actual.as_os_str().as_encoded_bytes(),
            )),
            crate::preserves_rail::u64_value(5),
            crate::preserves_rail::string("capacity-and-reopen-only-not-mttr-or-latency"),
        ]);
        let out = std::env::var("MOLTEN_CONTENT_RECEIPT_DIR").expect("explicit receipt directory");
        let namespace = crate::node_state::Scope::open_existing(
            crate::node_state::NamespaceKind::Receipts,
            std::path::Path::new(&out),
        )
        .expect("receipt authority");
        namespace
            .write(
                &crate::node_state::RelativePath::parse("nvme-path.preserves").expect("locator"),
                &crate::preserves_rail::canonical_bytes(&value).expect("canonical receipt"),
            )
            .expect("persist measured facts");
    }
}
