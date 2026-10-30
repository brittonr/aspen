//! Capability-local, no-follow metadata admission for the exact Mantle handoff.
use std::io::Read;

use cap_fs_ext::DirExt;
use cap_fs_ext::OpenOptionsFollowExt;

pub fn read(root: &cap_std::fs::Dir, path: &str) -> super::Result<Vec<u8>> {
    let parts: Vec<_> = path.split('/').collect();
    if parts.is_empty() || parts.len() > 64 || parts.iter().any(|part| part.is_empty() || *part == "." || *part == "..")
    {
        return Err("invalid package member path".into());
    }
    let mut parent = root.try_clone().map_err(super::failure)?;
    for part in &parts[..parts.len() - 1] {
        parent = parent.open_dir_nofollow(part).map_err(super::failure)?;
    }
    let mut options = cap_std::fs::OpenOptions::new();
    options.read(true).follow(cap_fs_ext::FollowSymlinks::No);
    let file = parent.open_with(parts[parts.len() - 1], &options).map_err(super::failure)?;
    if !file.metadata().map_err(super::failure)?.is_file() {
        return Err("metadata member is not regular".into());
    }
    let mut bytes = Vec::new();
    file.take(16 * 1024 * 1024 + 1).read_to_end(&mut bytes).map_err(super::failure)?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err("metadata member exceeds byte bound".into());
    }
    Ok(bytes)
}

pub fn validate(manifest: &serde_json::Value) -> super::Result<()> {
    let source = &manifest["source"];
    for (field, expected) in [
        ("schema", "mantle-wrpc-reference-bundle-v1"),
        ("remote_interface", super::INTERFACE),
        ("remote_function", super::FUNCTION),
        ("client_entrypoint", "run"),
        ("expected_remote_result", "hello from Rust"),
        ("capability_root", "capability"),
    ] {
        if manifest[field].as_str() != Some(expected) {
            return Err(format!("Mantle manifest field {field} differs"));
        }
    }
    for (field, expected) in [
        ("revision", super::REVISION),
        ("release", "0.15.0"),
        ("wasmtime", "33.0.0"),
        ("wrpc_runtime_wasmtime", "0.29.0"),
        ("wrpc_transport", "0.28.4"),
        ("wit_bindgen", "0.36.0"),
        ("repository", "https://github.com/bytecodealliance/wrpc"),
        ("guest_build_owner", "Mantle"),
    ] {
        if source[field].as_str() != Some(expected) {
            return Err(format!("Mantle cohort field {field} differs"));
        }
    }
    if source["interface"].as_str() != Some(super::INTERFACE)
        || source["function"].as_str() != Some(super::FUNCTION)
        || source["result"].as_str() != Some("string")
        || source["parameters"] != serde_json::json!([])
    {
        return Err("source world/function/shape differs from pinned unary".into());
    }
    if manifest["expected_client_result"].as_u64() != Some(42)
        || source["composition_is_remote_rpc_evidence"].as_bool() != Some(false)
        || manifest["members"].as_array().is_none_or(|members| members.is_empty() || members.len() > 40_000)
    {
        return Err("incomplete or overclaimed Mantle reference bundle".into());
    }
    Ok(())
}

pub fn metadata(
    root: &cap_std::fs::Dir,
    manifest: &serde_json::Value,
    role: &str,
) -> super::Result<(Vec<u8>, crate::wasm_component::RuntimeProfile)> {
    let entry = &manifest[role];
    let expected_bundle = format!("{role}/materialization-bundle.json");
    let expected_profile = format!("{role}/runtime-profile.json");
    if entry["bundle"].as_str() != Some(expected_bundle.as_str())
        || entry["runtime_profile"].as_str() != Some(expected_profile.as_str())
    {
        return Err("Mantle role selects an unexpected bundle/profile".into());
    }
    let bundle_bytes = read(root, &expected_bundle)?;
    let bundle: serde_json::Value = serde_json::from_slice(&bundle_bytes).map_err(super::failure)?;
    let profile: crate::wasm_component::RuntimeProfile =
        serde_json::from_slice(&read(root, &expected_profile)?).map_err(super::failure)?;
    let profile_ref = crate::wasm_component::r#ref(&profile);
    let portable = entry["final_portable"].as_str().ok_or("manifest lacks final portable")?;
    let logical = bundle["final_portable"]["logical_path"].as_str().ok_or("bundle lacks final portable")?;
    if !portable.starts_with(&format!("{role}/"))
        || portable.split('/').any(|part| part == ".." || part.is_empty())
        || !logical.ends_with(&format!("/{portable}"))
        || manifest["comparison_only"]
            .as_array()
            .is_some_and(|paths| paths.iter().any(|path| path.as_str() == Some(portable)))
    {
        return Err("loose/composed artifact is not the exact standalone remote component".into());
    }
    if entry["runtime_profile_ref"].as_str() != Some(profile_ref.as_str())
        || bundle["bundle_identity_blake3"] != entry["bundle_identity_blake3"]
        || profile.toolchain.wasmtime != "33.0.0"
        || profile.toolchain.wasmtime_wasi != "33.0.0"
        || profile.toolchain.wit_bindgen != "0.36.0"
        || profile.features.component_async
        || profile.profile_id != "molten-wrpc-wasmtime33-unary-sync-v1"
        || profile.wit.package != "mantle:wrpc-pilot"
        || profile.wit.world != role
        || profile.resources.fuel != 10_000_000
        || profile.resources.max_memory_bytes != 67_108_864
        || profile.resources.max_instances != 64
        || profile.resources.max_tables != 16
        || profile.resources.max_memories != 16
        || profile.resources.max_table_elements != 65_536
        || profile.resources.max_stack_bytes != 1_048_576
        || !profile.allowed_wasi_interfaces.is_empty()
        || profile.allowed_imports
            != if role == "client" {
                vec![super::INTERFACE.to_string()]
            } else {
                Vec::new()
            }
        || profile.resources.max_component_bytes > 16 * 1024 * 1024
        || profile.resources.max_wit_bytes > 1024 * 1024
    {
        return Err("runtime cohort/profile differs from pinned sync wRPC".into());
    }
    Ok((bundle_bytes, profile))
}

pub fn negative_cases(root: &cap_std::fs::Dir) -> super::Result<Vec<&'static str>> {
    let bytes = read(root, "manifest.json")?;
    let original: serde_json::Value = serde_json::from_slice(&bytes).map_err(super::failure)?;
    let mut cases = Vec::new();
    for (field, value, label) in [
        ("revision", "wrong-revision", "cohort-drift"),
        ("guest_build_owner", "Molten", "local-guest-build"),
        ("result", "stream<string>", "unsupported-async-world"),
    ] {
        let mut altered = original.clone();
        altered["source"][field] = serde_json::Value::String(value.into());
        if validate(&altered).is_ok() {
            return Err(format!("admitted negative {label}"));
        }
        cases.push(label);
    }
    let mut incomplete = original.clone();
    incomplete["members"] = serde_json::json!([]);
    if validate(&incomplete).is_ok() {
        return Err("admitted incomplete member inventory".into());
    }
    cases.push("incomplete-bundle");
    let mut loose = original.clone();
    loose["client"]["final_portable"] = serde_json::json!("client/artifacts/runtime-composed.wasm");
    if metadata(root, &loose, "client").is_ok() {
        return Err("admitted composed artifact as remote client".into());
    }
    cases.push("mismatched-loose-component");
    let mut missing = original;
    missing["client"]["bundle"] = serde_json::Value::Null;
    if metadata(root, &missing, "client").is_ok() {
        return Err("admitted client without materialization bundle".into());
    }
    cases.push("missing-client-bundle");
    Ok(cases)
}
