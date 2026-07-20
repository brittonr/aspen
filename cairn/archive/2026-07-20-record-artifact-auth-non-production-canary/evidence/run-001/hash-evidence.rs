#!/usr/bin/env -S nix shell "github:nix-community/fenix?rev=4e49ef62eedaa5c149d62b63b3f53844e1cc45d7#complete.toolchain" nixpkgs#gcc nixpkgs#clang nixpkgs#mold -c cargo -q -Zscript
---
[package]
edition = "2024"

[dependencies]
blake3 = "=1.8.2"
---

use std::error::Error;
use std::path::{Path, PathBuf};

const INVENTORY_FILE: &str = "BLAKE3SUMS";
const MAX_EVIDENCE_FILE_BYTES: u64 = 16 * 1024 * 1024;

fn main() -> Result<(), Box<dyn Error>> {
    let root = std::env::args().nth(1).ok_or("usage: hash-evidence.rs ROOT")?;
    let root = PathBuf::from(root);
    let mut files = Vec::new();
    collect_files(&root, &root, &mut files)?;
    files.sort();
    for path in files {
        let relative = path.strip_prefix(&root)?;
        if relative == Path::new(INVENTORY_FILE) {
            continue;
        }
        let metadata = std::fs::symlink_metadata(&path)?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(format!("evidence member is not a regular file: {}", relative.display()).into());
        }
        if metadata.len() > MAX_EVIDENCE_FILE_BYTES {
            return Err(format!("evidence member exceeds bounded size: {}", relative.display()).into());
        }
        let bytes = std::fs::read(&path)?;
        println!("{}  {}", blake3::hash(&bytes).to_hex(), relative.display());
    }
    Ok(())
}

fn collect_files(root: &Path, directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(format!("symlink is forbidden in evidence root {}: {}", root.display(), path.display()).into());
        }
        if metadata.is_dir() {
            collect_files(root, &path, files)?;
        } else if metadata.is_file() {
            files.push(path);
        } else {
            return Err(format!("special file is forbidden in evidence root {}: {}", root.display(), path.display()).into());
        }
    }
    Ok(())
}
