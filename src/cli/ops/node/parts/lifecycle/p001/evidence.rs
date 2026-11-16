struct StatusOpenDenialInput<'a> {
    out: &'a std::path::Path,
    state_root: &'a std::path::Path,
    target: &'a str,
    startup_ref: &'a str,
    config_ref: &'a str,
    binding_ref: &'a str,
}

const PROCESS_STATUS_MAX_BYTES: u64 = 65_536;

fn capture_process_identity() -> molten::error::Result<molten::nixos_vm::VmProcessIdentity> {
    use std::fmt::Write;
    use std::io::Read;
    let file = std::fs::File::open("/proc/self/status").map_err(molten::error::Failure::from)?;
    let mut status = String::new();
    file.take(PROCESS_STATUS_MAX_BYTES + 1).read_to_string(&mut status).map_err(molten::error::Failure::from)?;
    if u64::try_from(status.len()).ok().is_none_or(|length| length > PROCESS_STATUS_MAX_BYTES) {
        return Err(molten::error::Failure::invalid_harness("process status exceeds VM observation bound"));
    }
    let field = |name: &str| -> molten::error::Result<&str> {
        status.lines().find_map(|line| line.strip_prefix(name))
            .ok_or_else(|| molten::error::Failure::invalid_harness(format!("process status missing {name}")))
    };
    let effective = |name: &str| -> molten::error::Result<u32> {
        field(name)?.split_whitespace().nth(1)
            .ok_or_else(|| molten::error::Failure::invalid_harness(format!("process status missing effective {name}")))?
            .parse().map_err(|_| molten::error::Failure::invalid_harness(format!("invalid effective {name}")))
    };
    let group_text = field("Groups:")?.trim();
    let mut groups = group_text.split_whitespace().take(molten::nixos_vm::MAX_VM_EFFECTIVE_GROUPS + 1)
        .map(|group| group.parse::<u32>().map_err(|_| molten::error::Failure::invalid_harness("invalid process group")))
        .collect::<molten::error::Result<Vec<_>>>()?;
    if groups.len() > molten::nixos_vm::MAX_VM_EFFECTIVE_GROUPS {
        return Err(molten::error::Failure::invalid_harness("effective group count exceeds VM bound"));
    }
    groups.sort_unstable();
    groups.dedup();
    let mut normalized_groups = String::with_capacity(group_text.len().max(1));
    for group in groups {
        if !normalized_groups.is_empty() { normalized_groups.push(','); }
        write!(&mut normalized_groups, "{group}")
            .map_err(|_| molten::error::Failure::invalid_harness("cannot encode process groups"))?;
    }
    if normalized_groups.is_empty() { normalized_groups.push('-'); }
    let cap_eff = field("CapEff:")?.trim().to_string();
    u64::from_str_radix(&cap_eff, 16).map_err(|_| molten::error::Failure::invalid_harness("invalid process capabilities"))?;
    Ok(molten::nixos_vm::VmProcessIdentity {
        uid: effective("Uid:")?,
        gid: effective("Gid:")?,
        groups: normalized_groups,
        cap_eff,
    })
}

fn capture_process_executable_ref() -> molten::error::Result<String> {
    use std::io::Read;
    // Linux procfs pins the image executing this attempt, not a later replacement of its pathname.
    let file = std::fs::File::open("/proc/self/exe").map_err(molten::error::Failure::from)?;
    let maximum = molten::node_state::MAX_EXECUTABLE_BYTES;
    if file.metadata().map_err(molten::error::Failure::from)?.len() > maximum {
        return Err(molten::error::Failure::invalid_harness("executing image exceeds observation bound"));
    }
    let mut reader = file.take(maximum + 1);
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0u8; 16_384];
    let mut total = 0u64;
    loop {
        let bytes = reader.read(&mut buffer).map_err(molten::error::Failure::from)?;
        if bytes == 0 { break; }
        total += u64::try_from(bytes).map_err(|_| molten::error::Failure::invalid_harness("image read size overflow"))?;
        if total > maximum {
            return Err(molten::error::Failure::invalid_harness("executing image exceeds observation bound"));
        }
        hasher.update(&buffer[..bytes]);
    }
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}
