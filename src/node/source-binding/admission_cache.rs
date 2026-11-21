//! Guest-only cache of a successful source-gate admission.
//!
//! The key binds the exact trust bytes, binding, executing binary, and measured
//! artifacts. A hit skips repeated Nix signature, closure, and gate validation in
//! the same node root; retained inputs and the executing binary are still checked.
//! The full producer profile never reads or writes this cache.

use super::*;

const CACHE_PATH: &str = "cache/source-gate-admission.json";
const CACHE_SCHEMA: &str = "molten-guest-source-admission-cache-v1";
const CACHE_MAX_BYTES: u64 = 65_536;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    schema: String,
    key: String,
    binding_ref: String,
    executing_ref: String,
    gate_ref: String,
    gate_value: String,
}

pub(super) fn key(
    trust_bytes: &[u8],
    binding_ref: &str,
    executing_identity: &str,
    artifacts: &[molten_core::startup_binding::Artifact],
) -> crate::error::Result<String> {
    let mut fields = vec![
        crate::preserves_rail::string(&crate::preserves_rail::content_ref_from_bytes(trust_bytes)),
        crate::preserves_rail::string(binding_ref),
        crate::preserves_rail::string(executing_identity),
    ];
    for artifact in artifacts {
        fields.push(crate::preserves_rail::string(&artifact.role));
        fields.push(crate::preserves_rail::string(&artifact.content_ref));
    }
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
        "molten-guest-source-admission-cache-key-v1",
        fields,
    ))
}

pub(super) fn load(root: &crate::node_state::DirectoryRoot, key: &str) -> Option<Accepted> {
    let path = crate::node_state::RelativePath::parse(CACHE_PATH).ok()?;
    let bytes = root.read(&path, CACHE_MAX_BYTES).ok()?;
    let entry: Entry = serde_json::from_slice(&bytes).ok()?;
    if entry.schema != CACHE_SCHEMA || entry.key != key {
        return None;
    }
    let gate_value = crate::preserves_rail::parse_text(&entry.gate_value).ok()?;
    accepted(entry.binding_ref, entry.gate_ref, gate_value, &entry.executing_ref).ok()
}

pub(super) fn record(
    root: &crate::node_state::DirectoryRoot,
    key: &str,
    executing_ref: &str,
    accepted: &Accepted,
) -> crate::error::Result<()> {
    let directory = crate::node_state::RelativePath::parse("cache")?;
    root.create_dir_all(&directory)?;
    let path = crate::node_state::RelativePath::parse(CACHE_PATH)?;
    let bytes = serde_json::to_vec(&Entry {
        schema: CACHE_SCHEMA.to_string(),
        key: key.to_string(),
        binding_ref: accepted.binding_ref.clone(),
        executing_ref: executing_ref.to_string(),
        gate_ref: accepted.gate_ref.clone(),
        gate_value: crate::preserves_rail::to_text(&accepted.gate_value)?,
    })
    .map_err(|error| invalid(format!("guest admission cache encode: {error}")))?;
    if bytes.len() as u64 > CACHE_MAX_BYTES {
        return Err(invalid("guest admission cache exceeds its bound"));
    }
    root.write(&path, &bytes)?;
    Ok(())
}

/// Cheap guest identity for the executing file: canonical path and length. The first
/// full admission in a node root measures the exact content and records it in the entry.
pub(super) fn executing_identity() -> crate::error::Result<String> {
    let path = std::fs::canonicalize("/proc/self/exe").map_err(crate::error::Failure::from)?;
    let length = std::fs::metadata(&path).map_err(crate::error::Failure::from)?.len();
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
        "molten-guest-executing-identity-v1",
        vec![
            crate::preserves_rail::string(&path.to_string_lossy()),
            crate::preserves_rail::u64_value(length),
        ],
    ))
}
