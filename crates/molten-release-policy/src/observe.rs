use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use molten_core::release_dependency::ArchiveReceiptObservation;
use molten_core::release_dependency::DependencyObservation;
use molten_core::release_dependency::DistributionObservation;
use molten_core::release_dependency::EvidenceFileObservation;
use molten_core::release_dependency::ResolvedPackageIdentity;
use serde_json::Value as JsonValue;
use toml::Value as TomlValue;

use crate::profile::DependencyRow;
use crate::profile::Profile;

const GIT_SOURCE_PREFIX: &str = "git+";
const REVISION_FRAGMENT_SEPARATOR: char = '#';
const SOURCE_QUERY_SEPARATOR: char = '?';
const MAX_EVIDENCE_FILE_BYTES: u64 = 1_048_576;

pub struct RepositoryObservation {
    pub dependencies: Vec<DependencyObservation>,
    pub resolved_package_identities: Vec<ResolvedPackageIdentity>,
    pub unprofiled_git_dependencies: Vec<String>,
    pub archive_receipts: Vec<ArchiveReceiptObservation>,
    pub distribution: DistributionObservation,
}

pub fn observe_repository(
    root: &Path,
    profile: &Profile,
    evidence_sources: &BTreeMap<String, PathBuf>,
) -> Result<RepositoryObservation, String> {
    let manifest = parse_toml_file(&root.join("Cargo.toml"))?;
    let lock = parse_toml_file(&root.join("Cargo.lock"))?;
    let flake_lock = parse_json_file(&root.join("flake.lock"))?;
    let dependencies = observe_dependencies(profile, &manifest, &lock, &flake_lock);
    let resolved_package_identities = resolved_git_identities(&lock);
    let unprofiled_git_dependencies = unprofiled_git_dependencies(profile, &manifest);
    let archive_receipts = observe_archives(profile, &flake_lock, evidence_sources)?;
    let distribution = observe_distribution(root, profile);
    Ok(RepositoryObservation {
        dependencies,
        resolved_package_identities,
        unprofiled_git_dependencies,
        archive_receipts,
        distribution,
    })
}

fn parse_toml_file(path: &Path) -> Result<TomlValue, String> {
    let contents = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    toml::from_str(&contents).map_err(|error| format!("{}: {error}", path.display()))
}

fn parse_json_file(path: &Path) -> Result<JsonValue, String> {
    let contents = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    serde_json::from_str(&contents).map_err(|error| format!("{}: {error}", path.display()))
}

fn observe_dependencies(
    profile: &Profile,
    manifest: &TomlValue,
    lock: &TomlValue,
    flake_lock: &JsonValue,
) -> Vec<DependencyObservation> {
    profile.dependencies.iter().map(|row| observe_dependency(row, manifest, lock, flake_lock)).collect()
}

fn observe_dependency(
    row: &DependencyRow,
    manifest: &TomlValue,
    lock: &TomlValue,
    flake_lock: &JsonValue,
) -> DependencyObservation {
    let manifest_entry = manifest_dependency(manifest, row);
    let manifest_source_coordinate =
        manifest_entry.and_then(|entry| entry.get("git")).and_then(TomlValue::as_str).map(str::to_owned);
    let manifest_revision =
        manifest_entry.and_then(|entry| entry.get("rev")).and_then(TomlValue::as_str).map(str::to_owned);
    let lock_identity = find_lock_identity(lock, &row.package_name, &row.package_version);
    let package_version = lock_identity
        .as_ref()
        .map_or_else(|| row.package_version.clone(), |identity| identity.package_version.clone());
    let package_name = lock_identity
        .as_ref()
        .map_or_else(|| row.package_name.clone(), |identity| identity.package_name.clone());
    DependencyObservation {
        manifest_dependency: row.manifest_dependency.clone(),
        package_name,
        package_version,
        manifest_source_coordinate,
        manifest_revision,
        lock_source_coordinate: lock_identity.as_ref().map(|identity| identity.source_coordinate.clone()),
        lock_revision: lock_identity.as_ref().map(|identity| identity.immutable_revision.clone()),
        nix_input: row.nix_input.clone(),
        nix_revision: flake_input_revision(flake_lock, &row.nix_input),
    }
}

fn manifest_dependency<'a>(manifest: &'a TomlValue, row: &DependencyRow) -> Option<&'a toml::Table> {
    let section = match row.disposition.as_str() {
        "runtime" | "optional-runtime" => "dependencies",
        "development" => "dev-dependencies",
        _ => return None,
    };
    manifest
        .get(section)
        .and_then(TomlValue::as_table)
        .and_then(|dependencies| dependencies.get(&row.manifest_dependency))
        .and_then(TomlValue::as_table)
}

fn find_lock_identity(lock: &TomlValue, package_name: &str, package_version: &str) -> Option<ResolvedPackageIdentity> {
    lock.get("package")
        .and_then(TomlValue::as_array)?
        .iter()
        .filter_map(parse_lock_identity)
        .find(|identity| identity.package_name == package_name && identity.package_version == package_version)
}

fn resolved_git_identities(lock: &TomlValue) -> Vec<ResolvedPackageIdentity> {
    let Some(packages) = lock.get("package").and_then(TomlValue::as_array) else {
        return Vec::new();
    };
    packages.iter().filter_map(parse_lock_identity).collect()
}

fn parse_lock_identity(package: &TomlValue) -> Option<ResolvedPackageIdentity> {
    let table = package.as_table()?;
    let source = table.get("source")?.as_str()?;
    let (source_coordinate, immutable_revision) = parse_git_source(source)?;
    Some(ResolvedPackageIdentity {
        package_name: table.get("name")?.as_str()?.to_owned(),
        package_version: table.get("version")?.as_str()?.to_owned(),
        source_coordinate,
        immutable_revision,
    })
}

fn parse_git_source(source: &str) -> Option<(String, String)> {
    let source = source.strip_prefix(GIT_SOURCE_PREFIX)?;
    let (coordinate_with_query, revision) = source.rsplit_once(REVISION_FRAGMENT_SEPARATOR)?;
    let coordinate = coordinate_with_query.split(SOURCE_QUERY_SEPARATOR).next().unwrap_or_default();
    if coordinate.is_empty() || revision.is_empty() {
        return None;
    }
    Some((coordinate.to_owned(), revision.to_owned()))
}

fn flake_input_revision(flake_lock: &JsonValue, input: &str) -> Option<String> {
    flake_lock.get("nodes")?.get(input)?.get("locked")?.get("rev")?.as_str().map(str::to_owned)
}

fn unprofiled_git_dependencies(profile: &Profile, manifest: &TomlValue) -> Vec<String> {
    let profiled: BTreeSet<&str> = profile.dependencies.iter().map(|row| row.manifest_dependency.as_str()).collect();
    let mut unprofiled = Vec::new();
    for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
        let Some(entries) = manifest.get(section).and_then(TomlValue::as_table) else {
            continue;
        };
        for (name, value) in entries {
            let is_git = value.as_table().is_some_and(|table| table.contains_key("git"));
            if is_git && !profiled.contains(name.as_str()) {
                unprofiled.push(name.clone());
            }
        }
    }
    unprofiled.sort();
    unprofiled.dedup();
    unprofiled
}

fn observe_archives(
    profile: &Profile,
    flake_lock: &JsonValue,
    evidence_sources: &BTreeMap<String, PathBuf>,
) -> Result<Vec<ArchiveReceiptObservation>, String> {
    profile
        .archive_receipts
        .iter()
        .map(|receipt| {
            let source_root = evidence_sources
                .get(&receipt.id)
                .ok_or_else(|| format!("missing --evidence-source for {}", receipt.id))?;
            let archive_present = source_root.join(&receipt.archive_path).is_dir();
            let evidence_files = receipt
                .evidence_files
                .iter()
                .map(|file| EvidenceFileObservation {
                    relative_path: file.relative_path.clone(),
                    blake3: hash_file(&source_root.join(&file.relative_path)).ok(),
                })
                .collect();
            Ok(ArchiveReceiptObservation {
                id: receipt.id.clone(),
                nix_revision: flake_input_revision(flake_lock, &receipt.nix_input),
                archive_present,
                evidence_files,
            })
        })
        .collect()
}

fn hash_file(path: &Path) -> Result<String, String> {
    let metadata = fs::metadata(path).map_err(|error| format!("{}: {error}", path.display()))?;
    if metadata.len() > MAX_EVIDENCE_FILE_BYTES {
        return Err(format!(
            "{} exceeds the archive evidence limit of {MAX_EVIDENCE_FILE_BYTES} bytes",
            path.display()
        ));
    }
    let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

fn observe_distribution(root: &Path, profile: &Profile) -> DistributionObservation {
    let present_artifacts = profile
        .distribution
        .notice_artifacts
        .iter()
        .chain(profile.distribution.source_export_artifacts.iter())
        .filter(|relative_path| root.join(relative_path).is_file())
        .cloned()
        .collect();
    DistributionObservation { present_artifacts }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REVISION: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[test]
    fn git_source_parser_accepts_exact_https_source() {
        let source = format!("git+https://github.com/example/repo.git?rev={REVISION}#{REVISION}");
        let parsed = parse_git_source(&source).expect("exact source parses");
        assert_eq!(parsed.0, "https://github.com/example/repo.git");
        assert_eq!(parsed.1, REVISION);
    }

    #[test]
    fn git_source_parser_rejects_registry_and_missing_revision() {
        assert!(parse_git_source("registry+https://github.com/rust-lang/crates.io-index").is_none());
        assert!(parse_git_source("git+https://github.com/example/repo.git").is_none());
    }
}
