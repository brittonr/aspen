// SPDX-License-Identifier: AGPL-3.0-or-later
use std::io::Read;

use molten_batch_application::Error;
use molten_batch_core::Id;
use molten_orchestrator_codec as codec;
use molten_pure_net::HostBindings;
use serde::Serialize;

#[derive(Serialize)]
struct CompiledClosure {
    schema: &'static str,
    cargo_lock_bytes: Id,
    vendor_lock_bytes: Id,
    toolchain_policy_bytes: Id,
    architecture: &'static str,
    operating_system: &'static str,
    panic_mode: &'static str,
}

// r[impl molten.pure_net.identity]
fn dependency_closure() -> Result<Id, Error> {
    codec::identity("pure-net.compiled-dependency-closure.v1", &CompiledClosure {
        schema: "molten-orchestrator/pure-net-compiled-closure/v1",
        cargo_lock_bytes: codec::digest(include_bytes!("../../../Cargo.lock")),
        vendor_lock_bytes: codec::digest(include_bytes!("../../../../../vendor/sources.lock.json")),
        toolchain_policy_bytes: codec::digest(include_bytes!("../../../../../rust-toolchain.toml")),
        architecture: std::env::consts::ARCH,
        operating_system: std::env::consts::OS,
        panic_mode: if cfg!(panic = "abort") { "abort" } else { "unwind" },
    })
    .map_err(crate::invalid)
}

#[cfg(target_os = "linux")]
fn executable() -> Result<Id, Error> {
    // /proc/self/exe retains the actual executing inode, not its replaceable name.
    let mut image = std::fs::File::open("/proc/self/exe").map_err(crate::invalid)?;
    // The operator-selected executing image is already immutable kernel input;
    // bound streaming by its measured extent, not an arbitrary debug-size cap.
    let metadata = image.metadata().map_err(crate::invalid)?;
    let length = metadata.len();
    if !metadata.is_file() || length == 0 {
        return Err(Error::Bound);
    }
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut total = 0_u64;
    loop {
        let count = image.read(&mut buffer).map_err(crate::invalid)?;
        if count == 0 {
            break;
        }
        total = total.checked_add(count as u64).ok_or(Error::Bound)?;
        if total > length {
            return Err(Error::Denied);
        }
        hasher.update(&buffer[..count]);
    }
    if total != length {
        return Err(Error::Denied);
    }
    Ok(Id(*hasher.finalize().as_bytes()))
}

#[cfg(not(target_os = "linux"))]
fn executable() -> Result<Id, Error> {
    Err(Error::Unsupported)
}

pub fn pure_net_host_bindings() -> Result<HostBindings, Error> {
    Ok(HostBindings {
        executable: executable()?,
        dependency_closure: dependency_closure()?,
    })
}
