//! Compiled Cargo feature observation against the crate's own manifest.

pub(in crate::capability_qualification) fn features() -> crate::error::Result<Vec<String>> {
    let cohort = [
        ("chaoscontrol-conformance", cfg!(feature = "chaoscontrol-conformance")),
        ("doltlite-oracle", cfg!(feature = "doltlite-oracle")),
        ("component-rpc", cfg!(feature = "component-rpc")),
        ("component-rpc-telemetry", cfg!(feature = "component-rpc-telemetry")),
        ("executable-extents", cfg!(feature = "executable-extents")),
        ("job-dag-construction", cfg!(feature = "job-dag-construction")),
        ("marble-object-store", cfg!(feature = "marble-object-store")),
        ("profiler", cfg!(feature = "profiler")),
        ("profiler-alloc", cfg!(feature = "profiler-alloc")),
        ("profiler-disabled", cfg!(feature = "profiler-disabled")),
        ("profiler-perf", cfg!(feature = "profiler-perf")),
        ("spacewasm-pilot", cfg!(feature = "spacewasm-pilot")),
        ("world-snapshot-vm-cohort", cfg!(feature = "world-snapshot-vm-cohort")),
        ("content-store-object-storage", cfg!(feature = "content-store-object-storage")),
        ("test-fixtures", cfg!(feature = "test-fixtures")),
    ];
    let manifest: toml::Value = toml::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")))
        .map_err(|_| super::super::invalid("compiled Cargo feature manifest cannot be decoded"))?;
    let declared = manifest
        .get("features")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| super::super::invalid("compiled Cargo feature inventory missing"))?;
    if declared.len() != cohort.len()
        || declared.keys().any(|name| !cohort.iter().any(|(known, _)| name.as_str() == *known))
    {
        return Err(super::super::invalid(
            "compiled feature inventory drift; qualification producer requires explicit update",
        ));
    }
    let target_dependencies = manifest
        .get("target")
        .and_then(toml::Value::as_table)
        .into_iter()
        .flat_map(|targets| targets.values())
        .filter_map(|target| target.get("dependencies").and_then(toml::Value::as_table));
    for dependencies in
        manifest.get("dependencies").and_then(toml::Value::as_table).into_iter().chain(target_dependencies)
    {
        for (name, dependency) in dependencies {
            let is_optional = dependency.get("optional").and_then(toml::Value::as_bool) == Some(true);
            let is_explicitly_hidden = declared
                .values()
                .filter_map(toml::Value::as_array)
                .flatten()
                .filter_map(toml::Value::as_str)
                .any(|feature| feature.strip_prefix("dep:") == Some(name.as_str()));
            if is_optional && !declared.contains_key(name) && !is_explicitly_hidden {
                return Err(super::super::invalid(
                    "implicit optional Cargo feature has no compiled observation; qualification denied",
                ));
            }
        }
    }
    let mut result: Vec<_> =
        cohort.into_iter().filter(|(_, enabled)| *enabled).map(|(name, _)| name.to_string()).collect();
    result.sort();
    Ok(result)
}
