//! Published Mantle structure and capability-rooted private snapshot verification.
//!
//! Mantle owns bounded source-tree identity and copying; Molten measures the
//! remaining file roles. The published verifier sees the unchanged producer DTO
//! and fresh private snapshot, never mutable ambient source paths.

mod measurement;
mod staging;

const PUBLISHED_BUNDLE_SCHEMA: &str = "mantle-wasm-component-materialization-bundle-v1";
const MAX_BUNDLE_JSON_BYTES: usize = 16 * 1_024 * 1_024;
const MAX_DECLARED_MEMBERS: usize = 8_192;

type Denial = super::super::super::model::Denial;
type Result<T, E = Denial> = std::result::Result<T, E>;

/// A byte snapshot, not runtime, policy, or filesystem authority.
///
/// Construction calls the published structural and byte verifiers. Source
/// closures use Mantle's bounded capability-rooted copy; other roles are
/// independently remeasured. Only the selected component and WIT are retained.
#[derive(Debug)]
pub struct VerifiedBundle {
    component_bytes: Vec<u8>,
    wit_bytes: Vec<u8>,
    bundle_ref: String,
    profile_ref: String,
    mantle_evidence_refs: Vec<String>,
    structural_report: mantle_wasm_consumer_verifier::ConsumerVerificationReport,
    verification_report: mantle_wasm_consumer_verifier::ConsumerVerificationReport,
    member_observations: Vec<mantle_wasm_consumer_verifier::ConsumerMemberObservation>,
}

struct RequiredMember {
    role: &'static str,
    logical_path: String,
    digest: String,
    size: u64,
}

impl VerifiedBundle {
    /// Load a real bundle using an authorized source and staging-parent capability.
    /// `staging_leaf` is an explicitly admitted fresh child name, never bundle text.
    /// The private child is removed before success, and on every ordinary error.
    pub fn load(
        root: &cap_std::fs::Dir,
        staging_parent: &cap_std::fs::Dir,
        staging_leaf: &str,
        bundle_json: &[u8],
        profile: &super::super::super::model::RuntimeProfile,
    ) -> Result<Self> {
        staging::with_private_stage(staging_parent, staging_leaf, |staging| {
            Self::load_staged(root, staging, bundle_json, profile)
        })
    }

    fn load_staged(
        root: &cap_std::fs::Dir,
        staging: &cap_std::fs::Dir,
        bundle_json: &[u8],
        profile: &super::super::super::model::RuntimeProfile,
    ) -> Result<Self> {
        if bundle_json.len() > MAX_BUNDLE_JSON_BYTES {
            return Err(Denial::new("published Mantle bundle exceeds the JSON byte bound"));
        }
        let bundle = decode_bundle(bundle_json, mantle_wasm_consumer_verifier::verify_consumer_bundle)?;
        // Bound the public role collector before it can assert on hostile input.
        let member_count = 3_usize
            .saturating_add(bundle.wit_inputs.len())
            .saturating_add(bundle.package_inputs.len())
            .saturating_add(bundle.stage_receipts.len().saturating_mul(2));
        if member_count > MAX_DECLARED_MEMBERS {
            return Err(Denial::new("published Mantle bundle exceeds the member bound"));
        }
        let members: Vec<RequiredMember> = mantle_wasm_consumer_verifier::facade::declared_member_objects(&bundle)
            .into_iter()
            .map(|(role, object)| RequiredMember {
                role,
                logical_path: object.logical_path.clone(),
                digest: object.digest_blake3.to_string(),
                size: object.size_bytes,
            })
            .collect();
        let component_path = bundle.final_portable.logical_path.clone();
        let wit_path = bundle
            .wit_inputs
            .iter()
            .find(|object| format!("blake3:{}", object.digest_blake3) == profile.wit.source_ref)
            .map(|object| object.logical_path.clone());
        let is_portable_only = bundle.aot.is_none();
        let mut evidence_refs = vec![
            format!("blake3:{}", bundle.manifest_blake3),
            format!("blake3:{}", bundle.cohort_blake3),
            format!("blake3:{}", bundle.expected_octet_profile_blake3),
        ];
        let structural_report = mantle_wasm_consumer_verifier::verify_consumer_bundle(bundle.clone());
        require_structural(&structural_report)?;
        let profile_ref = super::super::super::profile::r#ref(profile);
        let expected_profile = structural_report
            .expected_runtime_profile_blake3
            .as_deref()
            .map(|digest| format!("blake3:{digest}"));
        if expected_profile.as_deref() != Some(profile_ref.as_str()) {
            return Err(Denial::new(
                "published Mantle bundle runtime profile differs from the admitted Molten profile",
            ));
        }
        if !is_portable_only {
            return Err(Denial::new("published Mantle bundle includes unsupported precompiled admission"));
        }
        let wit_path = wit_path.ok_or_else(|| Denial::new("published Mantle bundle lacks the admitted Molten WIT"))?;
        let measured = measurement::measure_members(
            root,
            staging,
            &members,
            measurement::Selection {
                component_path: &component_path,
                wit_path: &wit_path,
                profile,
            },
            &bundle,
        )?;
        let published_root = staging::consumer_root(staging)?;
        let verification_report = published_root
            .verify_bundle(bundle)
            .map_err(|_| Denial::new("published Mantle byte verifier could not resolve staged members"))?;
        require_complete(&structural_report, &verification_report, &measured.observations)?;
        let bundle_ref =
            structural_report
                .bundle_identity_blake3
                .as_deref()
                .map(|digest| format!("blake3:{digest}"))
                .ok_or_else(|| Denial::new("published Mantle structural verification omitted bundle identity"))?;
        evidence_refs.extend(members.iter().map(|member| format!("blake3:{}", member.digest)));
        Ok(Self {
            component_bytes: measured.component_bytes,
            wit_bytes: measured.wit_bytes,
            bundle_ref,
            profile_ref,
            mantle_evidence_refs: super::super::super::model::sorted_unique(&evidence_refs),
            structural_report,
            verification_report,
            member_observations: measured.observations,
        })
    }

    pub const fn component_bytes(&self) -> &[u8] {
        self.component_bytes.as_slice()
    }

    pub const fn wit_bytes(&self) -> &[u8] {
        self.wit_bytes.as_slice()
    }

    pub fn bundle_ref(&self) -> &str {
        &self.bundle_ref
    }

    pub fn mantle_evidence_refs(&self) -> &[String] {
        &self.mantle_evidence_refs
    }

    /// The actual published report: Structural only, never a fabricated Bytes layer.
    pub const fn structural_report(&self) -> &mantle_wasm_consumer_verifier::ConsumerVerificationReport {
        &self.structural_report
    }

    /// The actual published Structural+Bytes report over the private snapshot.
    pub const fn verification_report(&self) -> &mantle_wasm_consumer_verifier::ConsumerVerificationReport {
        &self.verification_report
    }

    /// Private snapshot observations, including Mantle's bounded source copy.
    pub fn member_observations(&self) -> &[mantle_wasm_consumer_verifier::ConsumerMemberObservation] {
        &self.member_observations
    }

    pub fn source<'a>(
        &'a self,
        envelope: &'a super::AdmissionEnvelope,
        consumer: super::super::super::model::Consumer,
    ) -> super::ArtifactSource<'a> {
        super::ArtifactSource::PublishedMantle {
            materialization: self,
            envelope,
            consumer,
        }
    }

    pub(super) fn admit(
        &self,
        profile: &super::super::super::model::RuntimeProfile,
        scope: super::super::super::model::EvidenceScope,
        envelope: &super::AdmissionEnvelope,
        consumer: super::super::super::model::Consumer,
    ) -> Result<super::Admission> {
        let mut blockers = Vec::new();
        if scope != super::super::super::model::EvidenceScope::Production {
            blockers.push("published Mantle source requires production evidence scope".to_string());
        }
        if self.profile_ref != super::super::super::profile::r#ref(profile) {
            blockers.push("published Mantle source was measured for another Molten profile".to_string());
        }
        super::linkage::validate(&self.bundle_ref, envelope, &mut blockers);
        if !blockers.is_empty() {
            return Err(Denial::from_blockers(blockers));
        }
        Ok(super::Admission {
            evidence_scope: scope,
            component_ref: super::super::super::model::content_ref(self.component_bytes()),
            wit_ref: super::super::super::model::content_ref(self.wit_bytes()),
            bundle_ref: Some(self.bundle_ref.clone()),
            consumer,
            profile_ref: self.profile_ref.clone(),
            mantle_evidence_refs: self.mantle_evidence_refs.clone(),
            valence_evidence_refs: envelope.valence_sidecar_refs.clone(),
            cairn_evidence_refs: envelope.cairn_acceptance_refs.clone(),
            policy_refs: envelope.policy_refs.clone(),
            authority_refs: envelope.authority_refs.clone(),
            resource_refs: envelope.resource_refs.clone(),
        })
    }
}

// Infer the producer's concrete DTO from its public verifier signature instead
// of introducing a second bundle schema or a direct implementation-core pin.
fn decode_bundle<T: serde::de::DeserializeOwned>(
    bytes: &[u8],
    _verifier: fn(T) -> mantle_wasm_consumer_verifier::ConsumerVerificationReport,
) -> Result<T> {
    serde_json::from_slice(bytes).map_err(|_| Denial::new("published Mantle bundle JSON is malformed"))
}

fn require_structural(report: &mantle_wasm_consumer_verifier::ConsumerVerificationReport) -> Result<()> {
    if report.status != mantle_wasm_consumer_verifier::ConsumerVerificationStatus::Verified
        || report.schema != mantle_wasm_consumer_verifier::CONSUMER_REPORT_SCHEMA
        || report.bundle_schema != PUBLISHED_BUNDLE_SCHEMA
        || report.layers_completed != [mantle_wasm_consumer_verifier::ConsumerLayer::Structural]
        || !report.blockers.is_empty()
        || report.non_claims != mantle_wasm_consumer_verifier::facade::non_claims_owned()
    {
        return Err(Denial::new("published Mantle structural verifier did not accept the bundle"));
    }
    Ok(())
}

fn require_complete(
    structural: &mantle_wasm_consumer_verifier::ConsumerVerificationReport,
    complete: &mantle_wasm_consumer_verifier::ConsumerVerificationReport,
    measured: &[mantle_wasm_consumer_verifier::ConsumerMemberObservation],
) -> Result<()> {
    if complete.status != mantle_wasm_consumer_verifier::ConsumerVerificationStatus::Verified
        || complete.schema != structural.schema
        || complete.bundle_schema != structural.bundle_schema
        || complete.bundle_identity_blake3 != structural.bundle_identity_blake3
        || complete.expected_runtime_profile_blake3 != structural.expected_runtime_profile_blake3
        || complete.non_claims != structural.non_claims
        || !complete.blockers.is_empty()
        || complete.layers_completed
            != [
                mantle_wasm_consumer_verifier::ConsumerLayer::Structural,
                mantle_wasm_consumer_verifier::ConsumerLayer::Bytes,
            ]
        || complete.members != measured
    {
        return Err(Denial::new(
            "published Mantle byte verification is blocked, rejected, or differs from source measurements",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
