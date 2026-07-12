#![allow(dead_code)]

use std::fs;

fn ambient_store_read(root: &std::path::Path, leaf: &str) {
    let child = root.join(leaf);
    let _ = std::fs::read(&child);
}

fn ambient_store_write(root: &std::path::Path, leaf: &str) {
    let child = root.join(leaf);
    let _ = fs::write(&child, b"ambient");
}

fn ambient_store_enumeration(root: &std::path::Path) {
    let _ = std::fs::read_dir(root);
}

fn ambient_store_reacquisition(root: &std::path::Path) {
    let _ = cap_std::fs::Dir::open_ambient_dir(root, cap_std::ambient_authority());
}
