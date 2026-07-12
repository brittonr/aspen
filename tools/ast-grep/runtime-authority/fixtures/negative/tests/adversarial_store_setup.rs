#![allow(dead_code)]

fn adversarial_setup(path: &std::path::Path) {
    let _ = std::fs::write(path, b"tampered fixture");
    let _ = std::fs::remove_file(path);
}
