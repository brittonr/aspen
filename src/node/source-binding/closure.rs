//! Actual NAR measurement and completeness checks for both compiler/build cohorts.

pub(super) fn validate(root: &cap_std::fs::Dir, trust: &super::Trust) -> crate::error::Result<()> {
    let bytes = super::measure::read(root, "build-closure", molten_core::startup_binding::closure::MAX_CLOSURE_BYTES)?;
    let closure: molten_core::startup_binding::closure::Closure =
        serde_json::from_slice(&bytes).map_err(|error| super::invalid(format!("measured build closure: {error}")))?;
    let mut graphs = Vec::with_capacity(molten_core::startup_binding::closure::ROLES.len());
    for role in molten_core::startup_binding::closure::ROLES {
        graphs.push(read_graph(root, role)?);
    }
    molten_core::startup_binding::closure::validate(&closure, &graphs)
        .map_err(|issue| super::invalid(format!("incomplete measured build closure: {issue:?}")))?;
    for batch in closure.paths.chunks(super::process::HASH_BATCH) {
        let paths: Vec<std::path::PathBuf> = batch.iter().map(|measured| measured.path.clone().into()).collect();
        let bytes = super::process::hash_batch(trust, &paths)?;
        let text = std::str::from_utf8(&bytes)
            .map_err(|error| super::invalid(format!("NAR measurement is not UTF-8: {error}")))?;
        let lines: Vec<&str> = text.lines().map(str::trim).filter(|line| !line.is_empty()).collect();
        if lines.len() != batch.len() {
            return Err(super::invalid("NAR measurement batch returned the wrong number of digests"));
        }
        for (measured, result) in batch.iter().zip(lines) {
            let digest = result.strip_prefix("blake3:").unwrap_or(result);
            if format!("blake3:{digest}") != measured.nar_ref {
                return Err(super::invalid(format!("source build NAR identity mismatch: {}", measured.path)));
            }
        }
    }
    Ok(())
}

fn read_graph(
    root: &cap_std::fs::Dir,
    role: &str,
) -> crate::error::Result<molten_core::startup_binding::closure::Graph> {
    let bytes =
        super::measure::read(root, &format!("{role}-closure"), molten_core::startup_binding::closure::MAX_GRAPH_BYTES)?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|error| super::invalid(format!("source build graph is not UTF-8: {error}")))?;
    let mut paths = std::collections::BTreeSet::new();
    for path in text.lines().filter(|line| line.starts_with("/nix/store/")) {
        if !molten_core::startup_binding::closure::is_store_root(path) {
            return Err(super::invalid("source build graph contains a non-root store path"));
        }
        paths.insert(path.to_string());
        if paths.len() > molten_core::startup_binding::closure::MAX_PATHS {
            return Err(super::invalid("source build graph path bound exceeded"));
        }
    }
    Ok(molten_core::startup_binding::closure::Graph {
        role: role.to_string(),
        graph_ref: crate::preserves_rail::content_ref_from_bytes(&bytes),
        paths: paths.into_iter().collect(),
    })
}
