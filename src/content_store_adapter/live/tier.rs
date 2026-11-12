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
    root: PathBuf,
    max_bytes: u64,
    used_bytes: u64,
}

impl LocalTier {
    pub fn open(root: &Path, max_bytes: u64, require_non_rotational: bool) -> Result<Self> {
        if max_bytes == 0 {
            return Err(Failure::invalid_harness("local tier capacity must be nonzero"));
        }
        std::fs::create_dir_all(root).map_err(Failure::from)?;
        if require_non_rotational {
            match non_rotational(root) {
                Some(true) => {}
                Some(false) => {
                    return Err(Failure::invalid_harness("local tier is not on a non-rotational device"));
                }
                None => {
                    return Err(Failure::invalid_harness("local tier device rotation cannot be observed"));
                }
            }
        }
        let mut used_bytes = 0_u64;
        let mut entries = 0_usize;
        for entry in std::fs::read_dir(root).map_err(Failure::from)? {
            let entry = entry.map_err(Failure::from)?;
            let metadata = entry.metadata().map_err(Failure::from)?;
            if metadata.is_file() {
                entries = entries.saturating_add(1);
                used_bytes = used_bytes
                    .checked_add(metadata.len())
                    .ok_or_else(|| Failure::invalid_harness("local tier size overflow"))?;
            }
        }
        if entries > MAX_TIER_ENTRIES || used_bytes > max_bytes {
            return Err(Failure::invalid_harness("local tier exceeds its configured bound"));
        }
        Ok(Self { root: root.to_path_buf(), max_bytes, used_bytes })
    }

    pub fn get(&self, object_ref: &str) -> Result<Option<Vec<u8>>> {
        let bytes = match std::fs::read(self.entry_path(object_ref)?) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(Failure::from(error)),
        };
        if bytes.len() as u64 > self.max_bytes
            || crate::preserves_rail::content_ref_from_bytes(&bytes) != object_ref
        {
            return Ok(None);
        }
        Ok(Some(bytes))
    }

    pub fn put(&mut self, object_ref: &str, bytes: &[u8]) -> Result<TierPut> {
        if crate::preserves_rail::content_ref_from_bytes(bytes) != object_ref {
            return Err(Failure::invalid_harness("local tier bytes do not match the object identity"));
        }
        let length = bytes.len() as u64;
        let next = self
            .used_bytes
            .checked_add(length)
            .ok_or_else(|| Failure::invalid_harness("local tier size overflow"))?;
        if next > self.max_bytes {
            return Ok(TierPut::RemoteOnly(length));
        }
        let path = self.entry_path(object_ref)?;
        let temporary = path.with_extension("tmp");
        {
            use std::io::Write;
            let mut file = std::fs::File::create(&temporary).map_err(Failure::from)?;
            file.write_all(bytes).map_err(Failure::from)?;
            file.sync_all().map_err(Failure::from)?;
        }
        std::fs::rename(&temporary, &path).map_err(Failure::from)?;
        self.used_bytes = next;
        Ok(TierPut::Cached(length))
    }

    pub fn remove(&mut self, object_ref: &str) -> Result<()> {
        let path = self.entry_path(object_ref)?;
        let length = std::fs::metadata(&path).map(|metadata| metadata.len()).unwrap_or(0);
        match std::fs::remove_file(&path) {
            Ok(()) => {
                self.used_bytes = self.used_bytes.saturating_sub(length);
                Ok(())
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(Failure::from(error)),
        }
    }

    fn entry_path(&self, object_ref: &str) -> Result<PathBuf> {
        let hex = object_ref
            .strip_prefix("blake3:")
            .ok_or_else(|| Failure::invalid_harness("local tier key is not a BLAKE3 reference"))?;
        if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()) {
            return Err(Failure::invalid_harness("local tier key is not canonical"));
        }
        Ok(self.root.join(format!("{hex}.chunk")))
    }
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

        std::fs::write(temporary.path().join(format!("{}.chunk", &first_ref[7..])), b"tampered")
            .expect("tamper");
        assert!(tier.get(&first_ref).expect("get").is_none());
    }
}
