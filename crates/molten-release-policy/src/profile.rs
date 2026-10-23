use std::path::Path;
use std::process::Command;

use molten_core::release_dependency::ArchiveReceiptExpectation;
use molten_core::release_dependency::CanonicalValenceAuthority;
use molten_core::release_dependency::DependencyExpectation;
use molten_core::release_dependency::DependencyPlane;
use molten_core::release_dependency::DistributionProfile;
use molten_core::release_dependency::EvidenceFileExpectation;
use molten_core::release_dependency::RegistryDependencyExpectation;
use molten_core::release_dependency::ReleaseDisposition;
use molten_core::release_dependency::SourceKind;
use molten_core::release_dependency::TransportPolicy;
use serde::Deserialize;

const PROFILE_SCHEMA: &str = "molten.release-dependency-profile.v2";

#[derive(Debug, Clone, Deserialize)]
pub struct Profile {
    pub schema: String,
    pub dependencies: Vec<DependencyRow>,
    pub registry_dependencies: Vec<RegistryDependencyRow>,
    pub canonical_valence: CanonicalValenceRow,
    pub archive_receipts: Vec<ArchiveReceiptRow>,
    pub distribution: DistributionRow,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DependencyRow {
    pub manifest_dependency: String,
    pub package_name: String,
    pub package_version: String,
    pub source_kind: String,
    pub source_coordinate: String,
    pub immutable_revision: String,
    pub nix_input: String,
    pub transport_policy: String,
    pub disposition: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegistryDependencyRow {
    pub manifest_dependency: String,
    pub package_name: String,
    pub package_version: String,
    pub source_kind: String,
    pub source_coordinate: String,
    pub checksum: String,
    pub transport_policy: String,
    pub plane: String,
    pub disposition: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CanonicalValenceRow {
    pub source_coordinate: String,
    pub immutable_revision: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArchiveReceiptRow {
    pub id: String,
    pub source_coordinate: String,
    pub source_revision: String,
    pub nix_input: String,
    pub archive_path: String,
    pub status: String,
    pub evidence_files: Vec<EvidenceFileRow>,
    pub non_claims: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EvidenceFileRow {
    pub relative_path: String,
    pub blake3: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DistributionRow {
    pub license: String,
    pub source_coordinate: String,
    pub source_revision: String,
    pub notice_artifacts: Vec<String>,
    pub source_export_artifacts: Vec<String>,
    pub non_claims: Vec<String>,
}

pub fn load_profile(path: &Path) -> Result<Profile, String> {
    let output = Command::new("nickel")
        .arg("export")
        .arg(path)
        .output()
        .map_err(|error| format!("failed to execute Nickel for {}: {error}", path.display()))?;
    if !output.status.success() {
        return Err(format!("Nickel rejected {}: {}", path.display(), String::from_utf8_lossy(&output.stderr).trim()));
    }
    let profile: Profile = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("invalid Nickel profile JSON from {}: {error}", path.display()))?;
    if profile.schema != PROFILE_SCHEMA {
        return Err(format!("unsupported profile schema: expected {PROFILE_SCHEMA}, observed {}", profile.schema));
    }
    Ok(profile)
}

impl Profile {
    pub fn dependency_expectations(&self) -> Result<Vec<DependencyExpectation>, String> {
        self.dependencies.iter().map(DependencyRow::to_core).collect()
    }

    pub fn registry_dependency_expectations(&self) -> Result<Vec<RegistryDependencyExpectation>, String> {
        self.registry_dependencies.iter().map(RegistryDependencyRow::to_core).collect()
    }

    pub fn canonical_valence_authority(&self) -> CanonicalValenceAuthority {
        CanonicalValenceAuthority {
            source_coordinate: self.canonical_valence.source_coordinate.clone(),
            immutable_revision: self.canonical_valence.immutable_revision.clone(),
        }
    }

    pub fn archive_expectations(&self) -> Vec<ArchiveReceiptExpectation> {
        self.archive_receipts.iter().map(ArchiveReceiptRow::to_core).collect()
    }

    pub fn distribution_profile(&self) -> DistributionProfile {
        DistributionProfile {
            license: self.distribution.license.clone(),
            source_coordinate: self.distribution.source_coordinate.clone(),
            source_revision: self.distribution.source_revision.clone(),
            notice_artifacts: self.distribution.notice_artifacts.clone(),
            source_export_artifacts: self.distribution.source_export_artifacts.clone(),
            non_claims: self.distribution.non_claims.clone(),
        }
    }
}

impl DependencyRow {
    fn to_core(&self) -> Result<DependencyExpectation, String> {
        let source_kind = match self.source_kind.as_str() {
            "git" => SourceKind::Git,
            other => return Err(format!("unsupported source kind for {}: {other}", self.manifest_dependency)),
        };
        let transport_policy = match self.transport_policy.as_str() {
            "https" => TransportPolicy::Https,
            "private-radicle" => TransportPolicy::PrivateRadicle,
            "ssh-pinned-nix-archive" => TransportPolicy::SshPinnedWithNixArchive,
            other => {
                return Err(format!("unsupported transport policy for {}: {other}", self.manifest_dependency));
            }
        };
        let disposition = parse_disposition(&self.manifest_dependency, &self.disposition)?;
        Ok(DependencyExpectation {
            manifest_dependency: self.manifest_dependency.clone(),
            package_name: self.package_name.clone(),
            package_version: self.package_version.clone(),
            source_kind,
            source_coordinate: self.source_coordinate.clone(),
            immutable_revision: self.immutable_revision.clone(),
            nix_input: self.nix_input.clone(),
            transport_policy,
            disposition,
        })
    }
}

impl RegistryDependencyRow {
    fn to_core(&self) -> Result<RegistryDependencyExpectation, String> {
        let source_kind = match self.source_kind.as_str() {
            "crates-io" => SourceKind::CratesIo,
            other => {
                return Err(format!("unsupported registry source kind for {}: {other}", self.manifest_dependency));
            }
        };
        let transport_policy = match self.transport_policy.as_str() {
            "crates.io" => TransportPolicy::CratesIo,
            other => {
                return Err(format!("unsupported registry transport policy for {}: {other}", self.manifest_dependency));
            }
        };
        let plane = match self.plane.as_str() {
            "implementation" => DependencyPlane::Implementation,
            "development" => DependencyPlane::Development,
            other => return Err(format!("unsupported dependency plane for {}: {other}", self.manifest_dependency)),
        };
        let disposition = parse_disposition(&self.manifest_dependency, &self.disposition)?;
        Ok(RegistryDependencyExpectation {
            manifest_dependency: self.manifest_dependency.clone(),
            package_name: self.package_name.clone(),
            package_version: self.package_version.clone(),
            source_kind,
            source_coordinate: self.source_coordinate.clone(),
            checksum: self.checksum.clone(),
            transport_policy,
            plane,
            disposition,
        })
    }
}

fn parse_disposition(manifest_dependency: &str, disposition: &str) -> Result<ReleaseDisposition, String> {
    match disposition {
        "runtime" => Ok(ReleaseDisposition::Runtime),
        "optional-runtime" => Ok(ReleaseDisposition::OptionalRuntime),
        "development" => Ok(ReleaseDisposition::Development),
        other => Err(format!("unsupported release disposition for {manifest_dependency}: {other}")),
    }
}

impl ArchiveReceiptRow {
    fn to_core(&self) -> ArchiveReceiptExpectation {
        ArchiveReceiptExpectation {
            id: self.id.clone(),
            source_coordinate: self.source_coordinate.clone(),
            source_revision: self.source_revision.clone(),
            nix_input: self.nix_input.clone(),
            archive_path: self.archive_path.clone(),
            status: self.status.clone(),
            evidence_files: self
                .evidence_files
                .iter()
                .map(|file| EvidenceFileExpectation {
                    relative_path: file.relative_path.clone(),
                    blake3: file.blake3.clone(),
                })
                .collect(),
            non_claims: self.non_claims.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROFILE_FIXTURE: &str = include_str!("../../../config/release-dependencies/fixtures/positive/exact-pins.ncl");

    fn registry_row() -> RegistryDependencyRow {
        RegistryDependencyRow {
            manifest_dependency: "marble".to_owned(),
            package_name: "marble".to_owned(),
            package_version: "16.0.2".to_owned(),
            source_kind: "crates-io".to_owned(),
            source_coordinate: "registry+https://github.com/rust-lang/crates.io-index".to_owned(),
            checksum: "c00d04008994fed635a52323bbe1dd8c0947292183585ddf8f1090615fafc3c6".to_owned(),
            transport_policy: "crates.io".to_owned(),
            plane: "implementation".to_owned(),
            disposition: "optional-runtime".to_owned(),
        }
    }

    #[test]
    fn registry_row_conversion_parses_plane_and_rejects_unknown_values() {
        let expectation = registry_row().to_core().expect("reviewed registry row converts");
        assert_eq!(expectation.plane, DependencyPlane::Implementation);
        assert_eq!(expectation.disposition, ReleaseDisposition::OptionalRuntime);
        assert_eq!(expectation.source_kind, SourceKind::CratesIo);
        assert_eq!(expectation.transport_policy, TransportPolicy::CratesIo);

        let mut plane = registry_row();
        plane.plane = "runtime".to_owned();
        let mut source_kind = registry_row();
        source_kind.source_kind = "git".to_owned();
        let mut transport = registry_row();
        transport.transport_policy = "https".to_owned();
        let mut disposition = registry_row();
        disposition.disposition = "implementation".to_owned();
        for row in [plane, source_kind, transport, disposition] {
            assert!(row.to_core().is_err(), "unknown value admitted: {row:?}");
        }
    }

    #[test]
    fn fixture_remains_an_explicit_profile_import() {
        assert!(PROFILE_FIXTURE.contains("profile.ncl"));
        assert!(!PROFILE_FIXTURE.contains("ssh://"));
    }
}
