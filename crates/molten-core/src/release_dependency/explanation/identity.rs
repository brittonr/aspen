//! Occurrence-exact identities for explanation snapshots and bindings.
//!
//! Identities are domain-separated BLAKE3 over length-framed fields in input
//! order, so row order and multiplicity are part of the snapshot identity.
//! Enumerations are encoded by stable labels, never by Rust layout or `Debug`.

use super::CollectionState;
use super::ExplanationBinding;
use crate::release_dependency::DependencyPlane;
use crate::release_dependency::ManifestTable;
use crate::release_dependency::ReleaseDependencyInput;
use crate::release_dependency::ReleaseDisposition;
use crate::release_dependency::SourceKind;
use crate::release_dependency::TransportPolicy;

const SNAPSHOT_IDENTITY_CONTEXT: &str = "onixresearch.molten.release-dependency.snapshot.identity.v1";
const BINDING_IDENTITY_CONTEXT: &str = "onixresearch.molten.release-dependency.explanation-binding.identity.v1";
const ABSENT_TAG: u8 = 0;
const PRESENT_TAG: u8 = 1;

struct FramedHasher(blake3::Hasher);

impl FramedHasher {
    fn new(context: &str) -> Self {
        Self(blake3::Hasher::new_derive_key(context))
    }

    fn length(&mut self, length: usize) {
        let encoded = u64::try_from(length).unwrap_or(u64::MAX).to_be_bytes();
        self.0.update(&encoded);
    }

    fn text(&mut self, value: &str) {
        self.length(value.len());
        self.0.update(value.as_bytes());
    }

    fn optional(&mut self, value: Option<&str>) {
        match value {
            None => {
                self.0.update(&[ABSENT_TAG]);
            }
            Some(value) => {
                self.0.update(&[PRESENT_TAG]);
                self.text(value);
            }
        }
    }

    fn flag(&mut self, value: bool) {
        self.0.update(&[u8::from(value)]);
    }

    fn relation(&mut self, label: &str, rows: usize) {
        self.text(label);
        self.length(rows);
    }

    fn texts<'a>(&mut self, label: &str, values: impl ExactSizeIterator<Item = &'a str>) {
        self.relation(label, values.len());
        for value in values {
            self.text(value);
        }
    }

    fn finish(self) -> String {
        format!("blake3:{}", self.0.finalize().to_hex())
    }
}

/// Identity of the exact input snapshot, including row order and duplicate rows.
pub(super) fn snapshot_identity(input: &ReleaseDependencyInput) -> String {
    let mut hasher = FramedHasher::new(SNAPSHOT_IDENTITY_CONTEXT);
    hash_dependency_rows(&mut hasher, input);
    hash_registry_rows(&mut hasher, input);
    hash_archive_rows(&mut hasher, input);
    hash_distribution(&mut hasher, input);
    hasher.finish()
}

fn hash_dependency_rows(hasher: &mut FramedHasher, input: &ReleaseDependencyInput) {
    hasher.relation("dependency", input.dependencies.len());
    for row in &input.dependencies {
        for value in [&row.manifest_dependency, &row.package_name, &row.package_version] {
            hasher.text(value);
        }
        hasher.text(source_kind_label(row.source_kind));
        for value in [&row.source_coordinate, &row.immutable_revision, &row.nix_input] {
            hasher.text(value);
        }
        hasher.text(transport_label(row.transport_policy));
        hasher.text(disposition_label(row.disposition));
    }
    hasher.relation("observation", input.observations.len());
    for row in &input.observations {
        for value in [&row.manifest_dependency, &row.package_name, &row.package_version] {
            hasher.text(value);
        }
        for value in [
            &row.manifest_source_coordinate,
            &row.manifest_revision,
            &row.lock_source_coordinate,
        ] {
            hasher.optional(value.as_deref());
        }
        hasher.optional(row.lock_revision.as_deref());
        hasher.text(&row.nix_input);
        hasher.optional(row.nix_revision.as_deref());
    }
    hasher.relation("resolved", input.resolved_package_identities.len());
    for row in &input.resolved_package_identities {
        for value in [
            &row.package_name,
            &row.package_version,
            &row.source_coordinate,
            &row.immutable_revision,
        ] {
            hasher.text(value);
        }
    }
    hasher.texts("unprofiled", input.unprofiled_git_dependencies.iter().map(String::as_str));
    hasher.text(&input.canonical_valence.source_coordinate);
    hasher.text(&input.canonical_valence.immutable_revision);
}

fn hash_registry_rows(hasher: &mut FramedHasher, input: &ReleaseDependencyInput) {
    hasher.relation("registry", input.registry_dependencies.len());
    for row in &input.registry_dependencies {
        for value in [&row.manifest_dependency, &row.package_name, &row.package_version] {
            hasher.text(value);
        }
        hasher.text(source_kind_label(row.source_kind));
        hasher.text(&row.source_coordinate);
        hasher.text(&row.checksum);
        hasher.text(transport_label(row.transport_policy));
        hasher.text(plane_label(row.plane));
        hasher.text(disposition_label(row.disposition));
    }
    hasher.relation("registry-observation", input.registry_observations.len());
    for row in &input.registry_observations {
        hasher.text(&row.manifest_dependency);
        hasher.optional(row.manifest_table.map(manifest_table_label));
        hasher.optional(row.manifest_version_requirement.as_deref());
        hasher.flag(row.is_manifest_optional);
        for value in [
            &row.lock_package_version,
            &row.lock_source_coordinate,
            &row.lock_checksum,
        ] {
            hasher.optional(value.as_deref());
        }
    }
}

fn hash_archive_rows(hasher: &mut FramedHasher, input: &ReleaseDependencyInput) {
    hasher.relation("archive", input.archive_receipts.len());
    for row in &input.archive_receipts {
        for value in [
            &row.id,
            &row.source_coordinate,
            &row.source_revision,
            &row.nix_input,
            &row.archive_path,
        ] {
            hasher.text(value);
        }
        hasher.text(&row.status);
        hasher.relation("archive-evidence", row.evidence_files.len());
        for file in &row.evidence_files {
            hasher.text(&file.relative_path);
            hasher.text(&file.blake3);
        }
        hasher.texts("archive-non-claim", row.non_claims.iter().map(String::as_str));
    }
    hasher.relation("archive-observation", input.archive_observations.len());
    for row in &input.archive_observations {
        hasher.text(&row.id);
        hasher.optional(row.nix_revision.as_deref());
        hasher.flag(row.archive_present);
        hasher.relation("archive-evidence-observation", row.evidence_files.len());
        for file in &row.evidence_files {
            hasher.text(&file.relative_path);
            hasher.optional(file.blake3.as_deref());
        }
    }
}

fn hash_distribution(hasher: &mut FramedHasher, input: &ReleaseDependencyInput) {
    let profile = &input.distribution;
    for value in [&profile.license, &profile.source_coordinate, &profile.source_revision] {
        hasher.text(value);
    }
    hasher.texts("distribution-notice", profile.notice_artifacts.iter().map(String::as_str));
    hasher.texts("distribution-source", profile.source_export_artifacts.iter().map(String::as_str));
    hasher.texts("distribution-non-claim", profile.non_claims.iter().map(String::as_str));
    let present = &input.distribution_observation.present_artifacts;
    hasher.texts("distribution-present", present.iter().map(String::as_str));
}

/// Identity of a report binding: snapshot, rule revision, declared collections, and limits.
pub(super) fn binding_identity(binding: &ExplanationBinding) -> String {
    let mut hasher = FramedHasher::new(BINDING_IDENTITY_CONTEXT);
    hasher.text(&binding.snapshot_ref);
    hasher.text(&binding.rule_revision);
    for (relation, state) in binding.collections.states() {
        hasher.text(relation.label());
        hasher.text(collection_label(state));
    }
    hasher.length(binding.limits.max_steps);
    hasher.length(binding.limits.max_facts);
    hasher.length(usize::from(binding.limits.max_depth));
    hasher.length(binding.limits.max_output_bytes);
    hasher.finish()
}

const fn collection_label(state: CollectionState) -> &'static str {
    match state {
        CollectionState::Completed => "completed",
        CollectionState::Incomplete => "incomplete",
        CollectionState::Unavailable => "unavailable",
    }
}

const fn source_kind_label(kind: SourceKind) -> &'static str {
    match kind {
        SourceKind::Git => "git",
        SourceKind::CratesIo => "crates-io",
    }
}

const fn transport_label(policy: TransportPolicy) -> &'static str {
    match policy {
        TransportPolicy::Https => "https",
        TransportPolicy::PrivateRadicle => "private-radicle",
        TransportPolicy::SshPinnedWithNixArchive => "ssh-pinned-with-nix-archive",
        TransportPolicy::CratesIo => "crates-io",
    }
}

const fn disposition_label(disposition: ReleaseDisposition) -> &'static str {
    match disposition {
        ReleaseDisposition::Runtime => "runtime",
        ReleaseDisposition::OptionalRuntime => "optional-runtime",
        ReleaseDisposition::Development => "development",
    }
}

const fn plane_label(plane: DependencyPlane) -> &'static str {
    match plane {
        DependencyPlane::Implementation => "implementation",
        DependencyPlane::Development => "development",
    }
}

const fn manifest_table_label(table: ManifestTable) -> &'static str {
    match table {
        ManifestTable::Dependencies => "dependencies",
        ManifestTable::DevDependencies => "dev-dependencies",
        ManifestTable::BuildDependencies => "build-dependencies",
    }
}
