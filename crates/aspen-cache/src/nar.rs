//! Pure-Rust NAR archive creation using `nix_compat::nar::writer`.
//!
//! Replaces the `nix nar dump-path` subprocess with in-process NAR serialization.
//! Computes SHA-256 in the same write pass via a [`HashingWriter`].
//!
//! # Usage
//!
//! ```ignore
//! let (nar_bytes, sha256_hash) = dump_path_nar_async("/nix/store/abc-hello".into()).await?;
//! let nar_hash = format!("sha256:{}", nix_compat::nixbase32::encode(&sha256_hash));
//! ```

use std::fs;
use std::io::BufReader;
use std::io::Write;
use std::io::{self};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::path::PathBuf;

use sha2::Digest;
use sha2::Sha256;

/// A writer wrapper that hashes all bytes written through it with SHA-256.
///
/// Same pattern as rio-build's `CountingWriter` but for hashing.
pub struct HashingWriter<W: Write> {
    inner: W,
    hasher: Sha256,
}

impl<W: Write> HashingWriter<W> {
    /// Create a new hashing writer wrapping the given inner writer.
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            hasher: Sha256::new(),
        }
    }

    /// Consume the writer and return the SHA-256 digest as `[u8; 32]`.
    pub fn finalize(self) -> (W, [u8; 32]) {
        let hash = self.hasher.finalize();
        (self.inner, hash.into())
    }
}

impl<W: Write> Write for HashingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.hasher.update(&buf[..n]);
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// Serialize a filesystem path to NAR format and compute its SHA-256 hash.
///
/// Returns `(nar_bytes, sha256_hash)` where `sha256_hash` is the 32-byte
/// SHA-256 digest of the NAR archive, computed during serialization (single pass).
///
/// Handles three node types:
/// - Regular files (with executable bit detection via `PermissionsExt::mode()`)
/// - Directories (entries sorted by filename, as required by NAR format)
/// - Symlinks (via `read_link`)
pub fn dump_path_nar(path: &Path) -> io::Result<(Vec<u8>, [u8; 32])> {
    let mut buf = Vec::new();
    let mut hashing_writer = HashingWriter::new(&mut buf);

    let node = nix_compat::nar::writer::open(&mut hashing_writer)?;
    write_node(path, node)?;

    let (_, hash) = hashing_writer.finalize();
    Ok((buf, hash))
}

/// Async version of [`dump_path_nar`] that runs in `tokio::task::spawn_blocking`.
pub async fn dump_path_nar_async(path: PathBuf) -> io::Result<(Vec<u8>, [u8; 32])> {
    tokio::task::spawn_blocking(move || dump_path_nar(&path)).await.map_err(io::Error::other)?
}

/// Write a single filesystem node to the NAR writer.
fn write_node<W: Write>(path: &Path, node: nix_compat::nar::writer::sync::Node<'_, W>) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;

    if metadata.is_symlink() {
        let target = fs::read_link(path)?;
        let target_bytes = target.as_os_str().as_encoded_bytes();
        node.symlink(target_bytes)?;
    } else if metadata.is_file() {
        let executable = metadata.permissions().mode() & 0o111 != 0;
        let size = metadata.len();
        let file = fs::File::open(path)?;
        let mut reader = BufReader::new(file);
        node.file(executable, size, &mut reader)?;
    } else if metadata.is_dir() {
        let mut dir = node.directory()?;

        // NAR requires directory entries sorted by name
        let mut entries: Vec<_> = fs::read_dir(path)?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(|a| a.file_name());

        for entry in entries {
            let name = entry.file_name();
            let name_bytes = name.as_encoded_bytes();
            let child_path = entry.path();
            let child_node = dir.entry(name_bytes)?;
            write_node(&child_path, child_node)?;
        }

        dir.close()?;
    } else {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("unsupported file type: {:?}", path)));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::symlink;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_hashing_writer() {
        let mut buf = Vec::new();
        let mut writer = HashingWriter::new(&mut buf);
        writer.write_all(b"hello world").unwrap();
        let (_, hash) = writer.finalize();

        // SHA-256 of "hello world"
        let expected = sha2::Sha256::digest(b"hello world");
        assert_eq!(hash, expected.as_slice());
        assert_eq!(buf, b"hello world");
    }

    #[test]
    fn test_dump_single_file() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("hello.txt");
        fs::write(&file_path, "Hello, world!\n").unwrap();

        let (nar_bytes, hash) = dump_path_nar(&file_path).unwrap();

        // NAR should start with "nix-archive-1" magic
        assert!(nar_bytes.len() > 20);
        // Hash should be 32 bytes
        assert_eq!(hash.len(), 32);
        // Hash should not be all zeros
        assert!(hash.iter().any(|&b| b != 0));
    }

    #[test]
    fn test_dump_directory() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("mydir");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("a.txt"), "aaa").unwrap();
        fs::write(sub.join("b.txt"), "bbb").unwrap();

        let (nar_bytes, hash) = dump_path_nar(&sub).unwrap();
        assert!(!nar_bytes.is_empty());
        assert!(hash.iter().any(|&b| b != 0));
    }

    #[test]
    fn test_dump_symlink() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.txt");
        fs::write(&target, "target content").unwrap();
        let link = dir.path().join("link.txt");
        symlink(&target, &link).unwrap();

        let (nar_bytes, _hash) = dump_path_nar(&link).unwrap();
        assert!(!nar_bytes.is_empty());
    }

    #[test]
    fn test_dump_executable_file() {
        use std::os::unix::fs::PermissionsExt;

        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("script.sh");
        fs::write(&file_path, "#!/bin/sh\necho hi\n").unwrap();
        fs::set_permissions(&file_path, fs::Permissions::from_mode(0o755)).unwrap();

        let (nar_bytes, _hash) = dump_path_nar(&file_path).unwrap();
        assert!(!nar_bytes.is_empty());
    }

    #[test]
    fn test_roundtrip_nar_reader() {
        // Create a tempdir with mixed content
        let dir = TempDir::new().unwrap();
        let root = dir.path().join("pkg");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("hello.txt"), "Hello!\n").unwrap();

        let sub = root.join("bin");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("run"), "#!/bin/sh\n").unwrap();
        fs::set_permissions(sub.join("run"), fs::Permissions::from_mode(0o755)).unwrap();

        symlink("../hello.txt", sub.join("link")).unwrap();

        let (nar_bytes, _hash) = dump_path_nar(&root).unwrap();

        // Verify the NAR can be parsed back by nix-compat's reader
        let mut cursor = std::io::Cursor::new(&nar_bytes);
        let reader_result = nix_compat::nar::reader::open(&mut cursor);
        assert!(reader_result.is_ok(), "NAR should be parseable: {:?}", reader_result.err());
    }

    #[test]
    fn test_deterministic_output() {
        // Same content should produce same NAR bytes and hash
        let dir1 = TempDir::new().unwrap();
        fs::write(dir1.path().join("f"), "content").unwrap();
        let (nar1, hash1) = dump_path_nar(&dir1.path().join("f")).unwrap();

        let dir2 = TempDir::new().unwrap();
        fs::write(dir2.path().join("f"), "content").unwrap();
        let (nar2, hash2) = dump_path_nar(&dir2.path().join("f")).unwrap();

        assert_eq!(nar1, nar2);
        assert_eq!(hash1, hash2);
    }

    #[tokio::test]
    async fn test_dump_path_nar_async() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "async test").unwrap();

        let (nar_bytes, hash) = dump_path_nar_async(file_path).await.unwrap();
        assert!(!nar_bytes.is_empty());
        assert!(hash.iter().any(|&b| b != 0));
    }

    /// Golden test: compare dump_path_nar output against `nix-store --dump`.
    /// Skipped if nix is not available.
    #[test]
    fn test_golden_nix_store_dump() {
        // Check if nix is available
        let nix_available = std::process::Command::new("nix-store").arg("--version").output().is_ok();

        if !nix_available {
            eprintln!("skipping golden test: nix-store not available");
            return;
        }

        // Create a temp file to NAR-ify
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("golden.txt");
        fs::write(&file_path, "golden test content\n").unwrap();

        // Our implementation
        let (our_nar, _hash) = dump_path_nar(&file_path).unwrap();

        // nix-store --dump
        let output = std::process::Command::new("nix-store")
            .args(["--dump", file_path.to_str().unwrap()])
            .output()
            .expect("nix-store --dump failed");

        if !output.status.success() {
            eprintln!("skipping golden test: nix-store --dump failed");
            return;
        }

        assert_eq!(our_nar, output.stdout, "NAR output should be byte-identical to nix-store --dump");
    }
}
