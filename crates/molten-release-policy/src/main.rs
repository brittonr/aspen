mod observe;
mod profile;

use std::collections::BTreeMap;
use std::env;
use std::path::Path;
use std::path::PathBuf;

use molten_core::release_dependency::ReleaseDependencyInput;
use molten_core::release_dependency::validate_release_dependencies;

const DEFAULT_PROFILE: &str = "config/release-dependencies/profile.ncl";
const EVIDENCE_SOURCE_SEPARATOR: char = '=';

struct Arguments {
    root: PathBuf,
    profile: PathBuf,
    evidence_sources: BTreeMap<String, PathBuf>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("release dependency validation failed:\n{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let arguments = parse_arguments(env::args_os().skip(1))?;
    let profile_path = resolve_path(&arguments.root, &arguments.profile);
    let profile = profile::load_profile(&profile_path)?;
    let observations = observe::observe_repository(&arguments.root, &profile, &arguments.evidence_sources)?;
    let input = ReleaseDependencyInput {
        dependencies: profile.dependency_expectations()?,
        observations: observations.dependencies,
        resolved_package_identities: observations.resolved_package_identities,
        unprofiled_git_dependencies: observations.unprofiled_git_dependencies,
        canonical_valence: profile.canonical_valence_authority(),
        archive_receipts: profile.archive_expectations(),
        archive_observations: observations.archive_receipts,
        distribution: profile.distribution_profile(),
        distribution_observation: observations.distribution,
    };
    let report = validate_release_dependencies(&input);
    let report_blake3 = blake3::hash(report.canonical_material.as_bytes()).to_hex().to_string();
    if report.is_valid() {
        println!(
            "release-dependency-profile decision=pass rows={} archives={} report_blake3={report_blake3}",
            input.dependencies.len(),
            input.archive_receipts.len()
        );
        return Ok(());
    }
    let diagnostics = report
        .diagnostics
        .iter()
        .map(|diagnostic| format!("{:?}: {}: {}", diagnostic.code, diagnostic.subject, diagnostic.message))
        .collect::<Vec<_>>()
        .join("\n");
    Err(format!("decision=deny report_blake3={report_blake3}\n{diagnostics}"))
}

fn parse_arguments(arguments: impl Iterator<Item = std::ffi::OsString>) -> Result<Arguments, String> {
    let mut root = PathBuf::from(".");
    let mut profile = PathBuf::from(DEFAULT_PROFILE);
    let mut evidence_sources = BTreeMap::new();
    let mut arguments = arguments;
    while let Some(argument) = arguments.next() {
        let argument = argument.into_string().map_err(|_| "arguments must be valid UTF-8".to_owned())?;
        match argument.as_str() {
            "--root" => root = next_path(&mut arguments, "--root")?,
            "--profile" => profile = next_path(&mut arguments, "--profile")?,
            "--evidence-source" => {
                let value = next_string(&mut arguments, "--evidence-source")?;
                let (id, path) = value
                    .split_once(EVIDENCE_SOURCE_SEPARATOR)
                    .ok_or_else(|| "--evidence-source must use ID=PATH".to_owned())?;
                if id.is_empty() || path.is_empty() {
                    return Err("--evidence-source ID and PATH must not be empty".to_owned());
                }
                if evidence_sources.insert(id.to_owned(), PathBuf::from(path)).is_some() {
                    return Err(format!("duplicate --evidence-source id: {id}"));
                }
            }
            "--help" | "-h" => {
                return Err(
                    "usage: molten-release-policy [--root PATH] [--profile PATH] --evidence-source ID=PATH ..."
                        .to_owned(),
                );
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    if evidence_sources.is_empty() {
        evidence_sources.insert("valence-integrity".to_owned(), root.join("../valence"));
        evidence_sources.insert("octet-cutover".to_owned(), root.join("../octet"));
    }
    Ok(Arguments {
        root,
        profile,
        evidence_sources,
    })
}

fn next_path(arguments: &mut impl Iterator<Item = std::ffi::OsString>, flag: &str) -> Result<PathBuf, String> {
    Ok(PathBuf::from(next_string(arguments, flag)?))
}

fn next_string(arguments: &mut impl Iterator<Item = std::ffi::OsString>, flag: &str) -> Result<String, String> {
    arguments
        .next()
        .ok_or_else(|| format!("{flag} requires a value"))?
        .into_string()
        .map_err(|_| format!("{flag} value must be valid UTF-8"))
}

fn resolve_path(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPECTED_EVIDENCE_SOURCE_COUNT: usize = 2;

    #[test]
    fn argument_parser_accepts_distinct_evidence_sources() {
        let arguments = [
            "--root",
            "fixture",
            "--evidence-source",
            "valence-integrity=../valence",
            "--evidence-source",
            "octet-cutover=../octet",
        ];
        let parsed = parse_arguments(arguments.into_iter().map(std::ffi::OsString::from)).expect("arguments parse");
        assert_eq!(parsed.root, Path::new("fixture"));
        assert_eq!(parsed.evidence_sources.len(), EXPECTED_EVIDENCE_SOURCE_COUNT);
    }

    #[test]
    fn argument_parser_rejects_duplicate_and_malformed_evidence_sources() {
        let duplicate = ["--evidence-source", "receipt=one", "--evidence-source", "receipt=two"];
        assert!(parse_arguments(duplicate.into_iter().map(std::ffi::OsString::from)).is_err());
        let malformed = ["--evidence-source", "missing-separator"];
        assert!(parse_arguments(malformed.into_iter().map(std::ffi::OsString::from)).is_err());
    }
}
