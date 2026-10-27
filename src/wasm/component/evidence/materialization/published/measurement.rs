//! Descriptor-relative source measurement and exact private snapshot copying.

use std::io::Read;
use std::io::Write;

use cap_fs_ext::DirExt;
use cap_fs_ext::OpenOptionsFollowExt;
use cap_std::fs::OpenOptionsExt;

// Linux O_NONBLOCK prevents a substituted FIFO from hanging before metadata.
const SOURCE_OPEN_NONBLOCK: i32 = 0o4_000;
const MAX_MEMBER_BYTES: u64 = 1_073_741_824;
const MAX_TOTAL_BYTES: u64 = 4_294_967_296;
const READ_BUFFER_BYTES: usize = 8_192;

pub(super) struct MeasuredMembers {
    pub(super) component_bytes: Vec<u8>,
    pub(super) wit_bytes: Vec<u8>,
    pub(super) observations: Vec<mantle_wasm_consumer_verifier::ConsumerMemberObservation>,
}

pub(super) struct Selection<'a> {
    pub(super) component_path: &'a str,
    pub(super) wit_path: &'a str,
    pub(super) profile: &'a super::super::super::super::model::RuntimeProfile,
}

pub(super) struct FileIdentity<'a> {
    pub(super) logical_path: &'a str,
    pub(super) digest: &'a str,
    pub(super) size: u64,
}

pub(super) fn measure_members(
    root: &cap_std::fs::Dir,
    staging: &cap_std::fs::Dir,
    members: &[super::RequiredMember],
    selection: Selection<'_>,
) -> super::Result<MeasuredMembers> {
    let Selection {
        component_path,
        wit_path,
        profile,
    } = selection;
    let declared = admit_members(members)?;
    let mut component_bytes = None;
    let mut wit_bytes = None;
    for (path, (digest, size)) in declared {
        let capture_bound = if path == component_path {
            Some(profile.resources.max_component_bytes)
        } else if path == wit_path {
            Some(profile.resources.max_wit_bytes)
        } else {
            None
        };
        if capture_bound.is_some_and(|bound| size > bound) {
            return Err(super::Denial::new("published Mantle executable member exceeds the admitted profile bound"));
        }
        let destination = super::staging::create_member(staging, relative_member(path)?)?;
        let captured = measure_member(
            root,
            FileIdentity {
                logical_path: path,
                digest,
                size,
            },
            capture_bound.is_some(),
            Some(destination),
        )?;
        if path == component_path {
            component_bytes = Some(captured);
        } else if path == wit_path {
            wit_bytes = Some(captured);
        }
    }
    let observations = members
        .iter()
        .map(|member| mantle_wasm_consumer_verifier::ConsumerMemberObservation {
            role: member.role.to_string(),
            digest_blake3: member.digest.clone(),
            size_bytes: member.size,
            status: Some(mantle_wasm_consumer_verifier::ConsumerMemberStatus::Matched),
        })
        .collect();
    Ok(MeasuredMembers {
        component_bytes: component_bytes
            .ok_or_else(|| super::Denial::new("published Mantle final portable member is missing"))?,
        wit_bytes: wit_bytes.ok_or_else(|| super::Denial::new("published Mantle WIT member is missing"))?,
        observations,
    })
}

fn admit_members(members: &[super::RequiredMember]) -> super::Result<std::collections::BTreeMap<&str, (&str, u64)>> {
    let mut declared = std::collections::BTreeMap::new();
    let mut total = 0_u64;
    for member in members {
        relative_member(&member.logical_path)?;
        if member.size > MAX_MEMBER_BYTES {
            return Err(super::Denial::new("published Mantle member exceeds the byte bound"));
        }
        if let Some((digest, size)) =
            declared.insert(member.logical_path.as_str(), (member.digest.as_str(), member.size))
        {
            if digest != member.digest || size != member.size {
                return Err(super::Denial::new("published Mantle member path has conflicting identities or lengths"));
            }
        } else {
            total = total
                .checked_add(member.size)
                .filter(|total| *total <= MAX_TOTAL_BYTES)
                .ok_or_else(|| super::Denial::new("published Mantle members exceed the aggregate byte bound"))?;
        }
    }
    Ok(declared)
}

fn relative_member(logical_path: &str) -> super::Result<&str> {
    let relative = logical_path
        .strip_prefix('/')
        .ok_or_else(|| super::Denial::new("published Mantle member lacks a logical root"))?;
    if relative
        .split('/')
        .any(|part| part.is_empty() || part == "." || part == ".." || part.contains('\0'))
    {
        return Err(super::Denial::new("published Mantle member path is not a confined logical path"));
    }
    Ok(relative)
}

pub(super) fn measure_member(
    root: &cap_std::fs::Dir,
    member: FileIdentity<'_>,
    capture: bool,
    destination: Option<cap_std::fs::File>,
) -> super::Result<Vec<u8>> {
    let FileIdentity {
        logical_path,
        digest,
        size,
    } = member;
    let relative = relative_member(logical_path)?;
    let mut parts = relative.split('/').peekable();
    let mut parent = root
        .try_clone()
        .map_err(|_| super::Denial::new("published Mantle root capability is unavailable"))?;
    while let Some(part) = parts.next() {
        if parts.peek().is_none() {
            let mut options = cap_std::fs::OpenOptions::new();
            options.read(true).follow(cap_fs_ext::FollowSymlinks::No).custom_flags(SOURCE_OPEN_NONBLOCK);
            let file = parent.open_with(part, &options).map_err(|_| {
                super::Denial::new("published Mantle member is unavailable under the source capability")
            })?;
            return measure_open_file(file, digest, size, capture, destination);
        }
        parent = parent
            .open_dir_nofollow(part)
            .map_err(|_| super::Denial::new("published Mantle member parent is unavailable or a symlink"))?;
    }
    Err(super::Denial::new("published Mantle member path has no leaf"))
}

fn measure_open_file(
    mut file: cap_std::fs::File,
    digest: &str,
    size: u64,
    capture: bool,
    mut destination: Option<cap_std::fs::File>,
) -> super::Result<Vec<u8>> {
    let metadata =
        file.metadata().map_err(|_| super::Denial::new("published Mantle member metadata is unavailable"))?;
    if !metadata.is_file() || metadata.len() != size {
        return Err(super::Denial::new("published Mantle member is not a regular file of the declared length"));
    }
    let mut hash = blake3::Hasher::new();
    let capacity = if capture {
        usize::try_from(size).map_err(|_| super::Denial::new("published Mantle capture exceeds the host bound"))?
    } else {
        0
    };
    let mut bytes = Vec::with_capacity(capacity);
    let mut read = 0_u64;
    let mut buffer = [0_u8; READ_BUFFER_BYTES];
    let mut bounded = std::io::Read::by_ref(&mut file).take(size.saturating_add(1));
    loop {
        let count = bounded.read(&mut buffer).map_err(|_| super::Denial::new("published Mantle member read failed"))?;
        if count == 0 {
            break;
        }
        read += count as u64;
        if read > size {
            return Err(super::Denial::new("published Mantle member grew beyond the declared length"));
        }
        hash.update(&buffer[..count]);
        if let Some(destination) = &mut destination {
            destination
                .write_all(&buffer[..count])
                .map_err(|_| super::Denial::new("published Mantle private member staging failed"))?;
        }
        if capture {
            bytes.extend_from_slice(&buffer[..count]);
        }
    }
    if read != size || hash.finalize().to_hex().as_str() != digest {
        return Err(super::Denial::new("published Mantle member bytes differ from the declared identity"));
    }
    Ok(bytes)
}
