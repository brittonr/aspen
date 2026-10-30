const MAX_ENTRIES: usize = 4096;
const MAX_COHORT_BYTES: u64 = 256 * 1024 * 1024;
const MAX_EXECUTABLE_BYTES: u64 = 2 * 1024 * 1024 * 1024;

pub(super) fn executing_ref() -> crate::error::Result<String> {
    let file = std::fs::File::open("/proc/self/exe").map_err(crate::error::Failure::from)?;
    let mut hasher = blake3::Hasher::new();
    hash_file(file, &mut hasher, MAX_EXECUTABLE_BYTES)?;
    Ok(crate::preserves_rail::content_ref_from_blake3_hash(hasher.finalize()))
}

pub(super) fn selected_executable_ref(root: &cap_std::fs::Dir) -> crate::error::Result<String> {
    if !root.symlink_metadata("bin").map_err(crate::error::Failure::from)?.is_dir() {
        return Err(super::invalid("selected output bin must be a regular directory"));
    }
    let binaries = root.open_dir("bin").map_err(crate::error::Failure::from)?;
    if !binaries.symlink_metadata("molten").map_err(crate::error::Failure::from)?.is_file() {
        return Err(super::invalid("selected output executable must be a regular file"));
    }
    let mut file = binaries.open("molten").map_err(crate::error::Failure::from)?.into_std();
    let mut magic = [0u8; 4];
    std::io::Read::read_exact(&mut file, &mut magic).map_err(crate::error::Failure::from)?;
    if magic != *b"\x7fELF" {
        return Err(super::invalid("selected output executable is not an ELF"));
    }
    std::io::Seek::rewind(&mut file).map_err(crate::error::Failure::from)?;
    let mut hasher = blake3::Hasher::new();
    hash_file(file, &mut hasher, MAX_EXECUTABLE_BYTES)?;
    Ok(crate::preserves_rail::content_ref_from_blake3_hash(hasher.finalize()))
}

pub(super) fn cohort_ref(root: &cap_std::fs::Dir, directory: &str) -> crate::error::Result<String> {
    if !root.symlink_metadata(directory).map_err(crate::error::Failure::from)?.is_dir() {
        return Err(super::invalid("source gate cohort must be a regular directory"));
    }
    let scope = root.open_dir(directory).map_err(crate::error::Failure::from)?;
    let mut files = collect(&scope)?;
    if files.is_empty() {
        return Err(super::invalid("empty source gate cohort"));
    }
    files.sort();
    let mut hasher = blake3::Hasher::new();
    let mut total = 0u64;
    for name in files {
        let file = scope.open(&name).map_err(crate::error::Failure::from)?;
        let size = file.metadata().map_err(crate::error::Failure::from)?.len();
        total = total.checked_add(size).ok_or_else(|| super::invalid("gate cohort size overflow"))?;
        if total > MAX_COHORT_BYTES {
            return Err(super::invalid("gate cohort size bound exceeded"));
        }
        hasher.update(&(name.len() as u64).to_le_bytes());
        hasher.update(name.as_bytes());
        hasher.update(&size.to_le_bytes());
        hash_file(file.into_std(), &mut hasher, size)?;
    }
    Ok(crate::preserves_rail::content_ref_from_blake3_hash(hasher.finalize()))
}

/// Bounds measured cohort nesting; the open-directory stack never exceeds this depth plus the root.
const MAX_PATH_SEPARATORS: usize = 32;
const MAX_PATH_BYTES: usize = 4096;

/// Depth-first walk over an explicit directory stack. Each level retains only its own entry
/// iterator.
fn collect(scope: &cap_std::fs::Dir) -> crate::error::Result<Vec<String>> {
    let mut files = Vec::new();
    let mut entries = 0usize;
    let mut stack = Vec::with_capacity(MAX_PATH_SEPARATORS + 2);
    stack.push((scope.entries().map_err(crate::error::Failure::from)?, String::new()));
    while let Some((iterator, prefix)) = stack.last_mut() {
        let Some(entry) = iterator.next() else {
            stack.pop();
            continue;
        };
        let entry = entry.map_err(crate::error::Failure::from)?;
        entries += 1;
        if entries > MAX_ENTRIES {
            return Err(super::invalid("gate cohort entry bound exceeded"));
        }
        let name = entry.file_name().into_string().map_err(|_| super::invalid("non-UTF8 gate member"))?;
        let relative = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        let kind = entry.file_type().map_err(crate::error::Failure::from)?;
        if kind.is_dir() {
            if relative.len() > MAX_PATH_BYTES
                || relative.bytes().filter(|byte| *byte == b'/').count() > MAX_PATH_SEPARATORS
            {
                return Err(super::invalid("gate cohort path bound exceeded"));
            }
            let child = entry.open_dir().map_err(crate::error::Failure::from)?;
            stack.push((child.entries().map_err(crate::error::Failure::from)?, relative));
        } else if kind.is_file() {
            if files.len() >= MAX_ENTRIES {
                return Err(super::invalid("gate cohort entry bound exceeded"));
            }
            files.push(relative);
        } else {
            return Err(super::invalid("gate cohort contains non-regular member"));
        }
    }
    Ok(files)
}

fn hash_file(mut file: std::fs::File, hasher: &mut blake3::Hasher, limit: u64) -> crate::error::Result<()> {
    let mut buffer = [0u8; 65536];
    let mut total = 0u64;
    loop {
        let count = std::io::Read::read(&mut file, &mut buffer).map_err(crate::error::Failure::from)?;
        if count == 0 {
            break;
        }
        total = total.checked_add(count as u64).ok_or_else(|| super::invalid("measured file size overflow"))?;
        if total > limit {
            return Err(super::invalid("measured file size bound exceeded"));
        }
        hasher.update(&buffer[..count]);
    }
    Ok(())
}

pub(super) fn read(root: &cap_std::fs::Dir, path: &str, limit: u64) -> crate::error::Result<Vec<u8>> {
    let metadata = root.symlink_metadata(path).map_err(crate::error::Failure::from)?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err(super::invalid("source evidence must be a bounded regular file"));
    }
    let file = root.open(path).map_err(crate::error::Failure::from)?;
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut std::io::Read::take(file, limit + 1), &mut bytes)
        .map_err(crate::error::Failure::from)?;
    if bytes.len() as u64 > limit {
        return Err(super::invalid("source evidence grew beyond its bound"));
    }
    Ok(bytes)
}
