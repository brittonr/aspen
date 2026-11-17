// SPDX-License-Identifier: AGPL-3.0-or-later
//! Linux whole-worker observations. These are not per-task sandbox limits.
use std::io::Read;

use molten_batch_application::Error;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ObservedProcessLimits {
    pub address_space_soft_bytes: u64,
    pub address_space_hard_bytes: u64,
    pub cpu_soft_seconds: u64,
    pub cpu_hard_seconds: u64,
}

fn pair(line: &str, name: &str, unit: &str) -> Result<Option<(u64, u64)>, Error> {
    let Some(rest) = line.strip_prefix(name) else {
        return Ok(None);
    };
    let mut fields = rest.split_whitespace();
    let soft = fields.next().and_then(|value| value.parse::<u64>().ok()).ok_or(Error::Denied)?;
    let hard = fields.next().and_then(|value| value.parse::<u64>().ok()).ok_or(Error::Denied)?;
    if fields.next() != Some(unit) || fields.next().is_some() || soft == 0 || soft > hard {
        return Err(Error::Denied);
    }
    Ok(Some((soft, hard)))
}

// r[impl molten.pure_net.bounds]
pub fn admit_process_limits(
    text: &str,
    address_space_bytes: u64,
    cpu_soft_seconds: u64,
    cpu_hard_seconds: u64,
) -> Result<ObservedProcessLimits, Error> {
    if text.len() > 8192 || address_space_bytes == 0 || cpu_soft_seconds == 0 || cpu_soft_seconds > cpu_hard_seconds {
        return Err(Error::Bound);
    }
    let mut address = None;
    let mut cpu = None;
    for line in text.lines() {
        if let Some(value) = pair(line, "Max address space", "bytes")? {
            if address.replace(value).is_some() {
                return Err(Error::Denied);
            }
        }
        if let Some(value) = pair(line, "Max cpu time", "seconds")? {
            if cpu.replace(value).is_some() {
                return Err(Error::Denied);
            }
        }
    }
    let (address_soft, address_hard) = address.ok_or(Error::Denied)?;
    let (cpu_soft, cpu_hard) = cpu.ok_or(Error::Denied)?;
    if address_hard > address_space_bytes || cpu_soft > cpu_soft_seconds || cpu_hard > cpu_hard_seconds {
        return Err(Error::Denied);
    }
    Ok(ObservedProcessLimits {
        address_space_soft_bytes: address_soft,
        address_space_hard_bytes: address_hard,
        cpu_soft_seconds: cpu_soft,
        cpu_hard_seconds: cpu_hard,
    })
}

#[cfg(target_os = "linux")]
pub fn observe_process_limits(
    address_space_bytes: u64,
    cpu_soft_seconds: u64,
    cpu_hard_seconds: u64,
) -> Result<ObservedProcessLimits, Error> {
    // Fixed kernel interface, not a caller-controlled application filesystem path.
    let mut file = std::fs::File::open("/proc/self/limits").map_err(crate::invalid)?;
    let mut bytes = [0_u8; 8193];
    let mut used = 0;
    loop {
        if used == bytes.len() {
            return Err(Error::Bound);
        }
        let count = file.read(&mut bytes[used..]).map_err(crate::invalid)?;
        if count == 0 {
            break;
        }
        used += count;
    }
    let text = std::str::from_utf8(&bytes[..used]).map_err(crate::invalid)?;
    admit_process_limits(text, address_space_bytes, cpu_soft_seconds, cpu_hard_seconds)
}

#[cfg(not(target_os = "linux"))]
pub fn observe_process_limits(
    _address_space_bytes: u64,
    _cpu_soft_seconds: u64,
    _cpu_hard_seconds: u64,
) -> Result<ObservedProcessLimits, Error> {
    Err(Error::Unsupported)
}

#[cfg(test)]
mod tests {
    use super::*;

    // r[verify molten.pure_net.bounds]
    #[test]
    fn finite_worker_caps_admit_the_exact_boundary_and_stricter_observations() {
        let exact = "Limit Soft Limit Hard Limit Units\nMax cpu time 60 61 seconds\nMax address space 4294967296 4294967296 bytes\n";
        let observed = admit_process_limits(exact, 4294967296, 60, 61).unwrap();
        assert_eq!(observed.cpu_hard_seconds, 61);
        assert_eq!(observed.address_space_hard_bytes, 4294967296);
        let tighter = "Max cpu time 59 60 seconds\nMax address space 2147483648 2147483648 bytes\n";
        assert_eq!(admit_process_limits(tighter, 4294967296, 60, 61).unwrap().cpu_soft_seconds, 59);
    }

    // r[verify molten.pure_net.bounds]
    #[test]
    fn unlimited_missing_duplicate_or_weaker_caps_never_admit_native_work() {
        for text in [
            "Max cpu time unlimited unlimited seconds\nMax address space 4294967296 4294967296 bytes\n",
            "Max cpu time 60 61 seconds\nMax address space unlimited unlimited bytes\n",
            "Max cpu time 61 62 seconds\nMax address space 4294967296 4294967296 bytes\n",
            "Max cpu time 60 61 seconds\nMax address space 4294967297 4294967297 bytes\n",
            "Max cpu time 60 61 seconds\n",
            "Max cpu time 60 61 seconds\nMax cpu time 1 1 seconds\nMax address space 4294967296 4294967296 bytes\n",
        ] {
            assert_eq!(admit_process_limits(text, 4294967296, 60, 61), Err(Error::Denied));
        }
    }
}
