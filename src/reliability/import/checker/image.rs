//! Bound and measure the exact executable used by both checker actions.

use std::os::fd::AsRawFd;

const MAX_EXECUTABLE_BYTES: u64 = 128 * 1024 * 1024;

pub(super) fn pin(checker: &super::Admitted) -> Result<std::fs::File, String> {
    let source = rustix::fs::open(
        &checker.executable,
        rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::CLOEXEC | rustix::fs::OFlags::NONBLOCK,
        rustix::fs::Mode::empty(),
    )
    .map(std::fs::File::from)
    .map_err(|e| e.to_string())?;
    let metadata = source.metadata().map_err(|e| e.to_string())?;
    if !metadata.is_file() {
        return Err("checker-not-file".into());
    }
    if !(1..=MAX_EXECUTABLE_BYTES).contains(&metadata.len()) {
        return Err("checker-executable-bound".into());
    }
    let source_fd_path = format!("/proc/self/fd/{}", source.as_raw_fd());
    rustix::fs::accessat(rustix::fs::CWD, source_fd_path, rustix::fs::Access::EXEC_OK, rustix::fs::AtFlags::EACCESS)
        .map_err(|e| format!("checker-executable-access:{e}"))?;

    // An open file pins its inode, not its contents. Hash the sealed image that both child
    // operations will execute, rather than measuring a file that can be rewritten in place.
    let fd = rustix::fs::memfd_create(
        "reliability-checker",
        rustix::fs::MemfdFlags::CLOEXEC | rustix::fs::MemfdFlags::ALLOW_SEALING | rustix::fs::MemfdFlags::EXEC,
    )
    .map_err(|e| e.to_string())?;
    let mut image = std::fs::File::from(fd);
    let copied =
        std::io::copy(&mut std::io::Read::take(source, metadata.len()), &mut image).map_err(|e| e.to_string())?;
    if copied != metadata.len() {
        return Err("checker-identity".into());
    }
    rustix::fs::fchmod(&image, rustix::fs::Mode::RUSR | rustix::fs::Mode::XUSR).map_err(|e| e.to_string())?;
    rustix::fs::fcntl_add_seals(
        &image,
        rustix::fs::SealFlags::WRITE
            | rustix::fs::SealFlags::GROW
            | rustix::fs::SealFlags::SHRINK
            | rustix::fs::SealFlags::SEAL,
    )
    .map_err(|e| e.to_string())?;
    let mut executable =
        std::fs::File::open(format!("/proc/self/fd/{}", image.as_raw_fd())).map_err(|e| e.to_string())?;
    drop(image);
    let mut hasher = blake3::Hasher::new();
    std::io::copy(&mut executable, &mut hasher).map_err(|e| e.to_string())?;
    if format!("blake3:{}", hasher.finalize().to_hex()) != checker.executable_blake3 {
        return Err("checker-identity".into());
    }
    Ok(executable)
}
