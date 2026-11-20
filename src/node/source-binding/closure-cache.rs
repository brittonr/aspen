//! Cohort-local cache of a successful full closure validation.
//!
//! Nix store paths are immutable and the key binds the exact output, trust key,
//! binding, selected executable, and every retained closure input. The cache
//! cannot stand in for signature, executable, or retained-input checks; those
//! still run on every invocation. A missing or mismatched cache re-runs the
//! full NAR closure measurement.

use super::*;

const CACHE_DIR: &str = "cache";
const CACHE_PATH: &str = "cache/source-closure-validation.json";
const CACHE_SCHEMA: &str = "molten-source-closure-cache-v1";
const CACHE_MAX_BYTES: u64 = 4_096;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    schema: String,
    key: String,
}

/// Bind the cache entry to every input whose change requires a new measurement.
pub(super) fn key(
    output: &cap_std::fs::Dir,
    trust: &Trust,
    binding_ref: &str,
    executing_ref: &str,
) -> crate::error::Result<String> {
    let mut retained = Vec::with_capacity(molten_core::startup_binding::closure::ROLES.len() + 1);
    retained.push(crate::preserves_rail::string(crate::preserves_rail::content_ref_from_bytes(&read(
        output,
        "build-closure",
        molten_core::startup_binding::closure::MAX_CLOSURE_BYTES,
    )?)));
    for role in molten_core::startup_binding::closure::ROLES {
        retained.push(crate::preserves_rail::string(crate::preserves_rail::content_ref_from_bytes(&read(
            output,
            &format!("{role}-closure"),
            molten_core::startup_binding::closure::MAX_GRAPH_BYTES,
        )?)));
    }
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("molten-source-closure-cache-key-v1", vec![
        crate::preserves_rail::string(trust.output.to_string_lossy().as_ref()),
        crate::preserves_rail::string(&trust.public_key),
        crate::preserves_rail::string(binding_ref),
        crate::preserves_rail::string(executing_ref),
        crate::preserves_rail::sequence(retained),
    ]))
}

fn read(output: &cap_std::fs::Dir, name: &str, maximum: u64) -> crate::error::Result<Vec<u8>> {
    measure::read(output, name, maximum)
}

pub(super) fn is_valid(root: &crate::node_state::DirectoryRoot, key: &str) -> bool {
    let Ok(path) = crate::node_state::RelativePath::parse(CACHE_PATH) else {
        return false;
    };
    let Ok(bytes) = root.read(&path, CACHE_MAX_BYTES) else {
        return false;
    };
    serde_json::from_slice::<Entry>(&bytes).is_ok_and(|entry| entry.schema == CACHE_SCHEMA && entry.key == key)
}

/// Best-effort cache publication. Failure never changes source-admission success.
pub(super) fn record(root: &crate::node_state::DirectoryRoot, key: &str) {
    let Ok(directory) = crate::node_state::RelativePath::parse(CACHE_DIR) else {
        return;
    };
    let Ok(path) = crate::node_state::RelativePath::parse(CACHE_PATH) else {
        return;
    };
    if root.create_dir_all(&directory).is_err() {
        return;
    }
    let Ok(bytes) = serde_json::to_vec(&Entry {
        schema: CACHE_SCHEMA.to_string(),
        key: key.to_string(),
    }) else {
        return;
    };
    if root.write(&path, &bytes).is_err() {
        return;
    }
}
