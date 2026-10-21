use std::io::Write;

use cap_fs_ext::OpenOptionsFollowExt;

// r[impl molten.audit_f03.recover_publication]
// An error after creation retains the partial member for explicit reconciliation.
pub(super) fn write_new_regular_file(
    dir: &cap_std::fs::Dir,
    path: &std::path::Path,
    bytes: &[u8],
) -> crate::error::Result<()> {
    let (parent, leaf) = super::filesystem::open_parent(dir, path, false)?;
    let mut options = cap_std::fs::OpenOptions::new();
    options.write(true).create_new(true).follow(cap_fs_ext::FollowSymlinks::No);
    let mut file = parent.open_with(leaf, &options).map_err(crate::error::Failure::from)?;
    if !file.metadata().map_err(crate::error::Failure::from)?.is_file() {
        return Err(super::invalid("new node state publication is not a regular file"));
    }
    file.write_all(bytes).map_err(crate::error::Failure::from)?;
    file.sync_all().map_err(crate::error::Failure::from)?;
    // Directory capabilities may hold O_PATH descriptors, which cannot be synchronized.
    parent
        .open(".")
        .map_err(crate::error::Failure::from)?
        .sync_all()
        .map_err(crate::error::Failure::from)
}
