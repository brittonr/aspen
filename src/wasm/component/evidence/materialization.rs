pub const MANTLE_COMPONENT_BUNDLE_SCHEMA: &str = "mantle.component-materialization-bundle.v1";
pub const COMPONENT_ADMISSION_ENVELOPE_SCHEMA: &str = "molten.component-admission-envelope.v1";
const MAX_COMPONENT_EVIDENCE_REFS: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterializedObjectIdentity {
    pub content_ref: String,
    pub byte_length: u64,
}

impl MaterializedObjectIdentity {
    pub fn measure(bytes: &[u8]) -> super::super::model::ComponentResult<Self> {
        let byte_length = u64::try_from(bytes.len()).map_err(|error| {
            super::super::model::ComponentDenial::new(format!("materialized object length is unsupported: {error}"))
        })?;
        Ok(Self {
            content_ref: super::super::model::content_ref(bytes),
            byte_length,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MantleComponentBundle {
    pub schema_id: String,
    pub bundle_ref: String,
    pub component: MaterializedObjectIdentity,
    pub wit: MaterializedObjectIdentity,
    pub artifact_kind: super::super::model::WasmArtifactKind,
    pub consumer: super::super::model::ComponentConsumer,
    pub expected_profile_id: String,
    pub expected_cohort_ref: String,
    pub build_cohort_ref: String,
    pub octet_report_ref: String,
    pub stage_receipt_refs: Vec<String>,
    pub embedded_admission_refs: Vec<String>,
    pub has_portable_bytes: bool,
    pub has_precompiled_bytes: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentAdmissionEnvelope {
    pub schema_id: String,
    pub bundle_ref: String,
    pub valence_sidecar_refs: Vec<String>,
    pub cairn_acceptance_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub authority_refs: Vec<String>,
    pub resource_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum ComponentArtifactSource<'a> {
    Mantle {
        bundle: &'a MantleComponentBundle,
        envelope: &'a ComponentAdmissionEnvelope,
        component_bytes: &'a [u8],
        wit_bytes: &'a [u8],
    },
    TestOnlyLoose {
        component_bytes: &'a [u8],
        wit_bytes: &'a [u8],
    },
}

impl<'a> ComponentArtifactSource<'a> {
    pub const fn component_bytes(self) -> &'a [u8] {
        match self {
            Self::Mantle { component_bytes, .. } | Self::TestOnlyLoose { component_bytes, .. } => component_bytes,
        }
    }

    pub const fn wit_bytes(self) -> &'a [u8] {
        match self {
            Self::Mantle { wit_bytes, .. } | Self::TestOnlyLoose { wit_bytes, .. } => wit_bytes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterializationAdmission {
    pub evidence_scope: super::super::model::EvidenceScope,
    pub component_ref: String,
    pub wit_ref: String,
    pub bundle_ref: Option<String>,
    pub consumer: super::super::model::ComponentConsumer,
    pub profile_ref: String,
    pub mantle_evidence_refs: Vec<String>,
    pub valence_evidence_refs: Vec<String>,
    pub cairn_evidence_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub authority_refs: Vec<String>,
    pub resource_refs: Vec<String>,
}

pub fn mantle_bundle_ref(bundle: &MantleComponentBundle) -> String {
    let mut lines = vec![
        format!("schema:{}", bundle.schema_id),
        format!("component-ref:{}", bundle.component.content_ref),
        format!("component-bytes:{}", bundle.component.byte_length),
        format!("wit-ref:{}", bundle.wit.content_ref),
        format!("wit-bytes:{}", bundle.wit.byte_length),
        format!("artifact-kind:{}", bundle.artifact_kind.as_str()),
        format!("consumer:{}", bundle.consumer.as_str()),
        format!("profile-id:{}", bundle.expected_profile_id),
        format!("cohort-ref:{}", bundle.expected_cohort_ref),
        format!("build-cohort-ref:{}", bundle.build_cohort_ref),
        format!("octet-report-ref:{}", bundle.octet_report_ref),
        format!("portable:{}", bundle.has_portable_bytes),
        format!("precompiled:{}", bundle.has_precompiled_bytes),
    ];
    lines.extend(
        super::super::model::sorted_unique(&bundle.stage_receipt_refs)
            .into_iter()
            .map(|value| format!("stage-receipt:{value}")),
    );
    lines.extend(
        super::super::model::sorted_unique(&bundle.embedded_admission_refs)
            .into_iter()
            .map(|value| format!("embedded-admission:{value}")),
    );
    super::super::model::content_ref(lines.join("\n").as_bytes())
}

pub fn verify_materialization(
    profile: &super::super::model::ComponentRuntimeProfile,
    requested_scope: super::super::model::EvidenceScope,
    source: ComponentArtifactSource<'_>,
) -> super::super::model::ComponentResult<MaterializationAdmission> {
    let component = MaterializedObjectIdentity::measure(source.component_bytes())?;
    let wit = MaterializedObjectIdentity::measure(source.wit_bytes())?;
    validate_byte_bounds(profile, &component, &wit)?;
    match source {
        ComponentArtifactSource::Mantle {
            bundle,
            envelope,
            component_bytes: _,
            wit_bytes: _,
        } => verify_bundle(BundleVerificationInput {
            profile,
            requested_scope,
            bundle,
            envelope,
            measured_component: &component,
            measured_wit: &wit,
        }),
        ComponentArtifactSource::TestOnlyLoose {
            component_bytes: _,
            wit_bytes: _,
        } => {
            if requested_scope != super::super::model::EvidenceScope::TestOnly {
                return Err(super::super::model::ComponentDenial::new(
                    "production component execution requires a Mantle materialization bundle",
                ));
            }
            Ok(MaterializationAdmission {
                evidence_scope: super::super::model::EvidenceScope::TestOnly,
                component_ref: component.content_ref,
                wit_ref: wit.content_ref,
                bundle_ref: None,
                consumer: super::super::model::ComponentConsumer::Actor,
                profile_ref: super::super::profile::component_profile_ref(profile),
                mantle_evidence_refs: Vec::new(),
                valence_evidence_refs: Vec::new(),
                cairn_evidence_refs: Vec::new(),
                policy_refs: Vec::new(),
                authority_refs: Vec::new(),
                resource_refs: Vec::new(),
            })
        }
    }
}

fn validate_byte_bounds(
    profile: &super::super::model::ComponentRuntimeProfile,
    component: &MaterializedObjectIdentity,
    wit: &MaterializedObjectIdentity,
) -> super::super::model::ComponentResult<()> {
    let mut blockers = Vec::new();
    if component.byte_length > profile.resources.max_component_bytes {
        blockers.push("component bytes exceed the admitted profile bound".to_string());
    }
    if wit.byte_length > profile.resources.max_wit_bytes {
        blockers.push("WIT bytes exceed the admitted profile bound".to_string());
    }
    if blockers.is_empty() {
        Ok(())
    } else {
        Err(super::super::model::ComponentDenial::from_blockers(blockers))
    }
}

struct BundleVerificationInput<'a> {
    profile: &'a super::super::model::ComponentRuntimeProfile,
    requested_scope: super::super::model::EvidenceScope,
    bundle: &'a MantleComponentBundle,
    envelope: &'a ComponentAdmissionEnvelope,
    measured_component: &'a MaterializedObjectIdentity,
    measured_wit: &'a MaterializedObjectIdentity,
}

fn verify_bundle(input: BundleVerificationInput<'_>) -> super::super::model::ComponentResult<MaterializationAdmission> {
    let mut blockers = Vec::new();
    if input.requested_scope != super::super::model::EvidenceScope::Production {
        blockers.push("Mantle production bundle must execute in production evidence scope".to_string());
    }
    if input.bundle.schema_id != MANTLE_COMPONENT_BUNDLE_SCHEMA {
        blockers.push("unsupported Mantle component bundle schema".to_string());
    }
    if input.bundle.bundle_ref != mantle_bundle_ref(input.bundle) {
        blockers.push("Mantle component bundle identity mismatch".to_string());
    }
    if &input.bundle.component != input.measured_component || &input.bundle.wit != input.measured_wit {
        blockers.push("Mantle component bundle object identity differs from remeasured bytes".to_string());
    }
    if input.bundle.artifact_kind != super::super::model::WasmArtifactKind::Component {
        blockers.push("Mantle component bundle is not classified as a component".to_string());
    }
    if input.bundle.expected_profile_id != input.profile.profile_id
        || input.bundle.expected_cohort_ref != super::super::profile::component_profile_ref(input.profile)
    {
        blockers.push("Mantle component bundle expected profile is stale or mismatched".to_string());
    }
    if input.bundle.wit.content_ref != input.profile.wit.source_ref {
        blockers.push("Mantle component bundle WIT identity does not match the admitted profile".to_string());
    }
    validate_bundle_refs(input.bundle, &mut blockers);
    validate_envelope(input.bundle, input.envelope, &mut blockers);
    if !input.bundle.has_portable_bytes || input.bundle.has_precompiled_bytes {
        blockers
            .push("initial component cohort admits portable bytes and rejects precompiled deserialization".to_string());
    }
    if !blockers.is_empty() {
        return Err(super::super::model::ComponentDenial::from_blockers(blockers));
    }
    Ok(MaterializationAdmission {
        evidence_scope: super::super::model::EvidenceScope::Production,
        component_ref: input.measured_component.content_ref.clone(),
        wit_ref: input.measured_wit.content_ref.clone(),
        bundle_ref: Some(input.bundle.bundle_ref.clone()),
        consumer: input.bundle.consumer,
        profile_ref: super::super::profile::component_profile_ref(input.profile),
        mantle_evidence_refs: mantle_evidence_refs(input.bundle),
        valence_evidence_refs: input.envelope.valence_sidecar_refs.clone(),
        cairn_evidence_refs: input.envelope.cairn_acceptance_refs.clone(),
        policy_refs: input.envelope.policy_refs.clone(),
        authority_refs: input.envelope.authority_refs.clone(),
        resource_refs: input.envelope.resource_refs.clone(),
    })
}

fn mantle_evidence_refs(bundle: &MantleComponentBundle) -> Vec<String> {
    let mut refs = vec![bundle.build_cohort_ref.clone(), bundle.octet_report_ref.clone()];
    refs.extend(bundle.stage_receipt_refs.clone());
    super::super::model::sorted_unique(&refs)
}

fn validate_bundle_refs(bundle: &MantleComponentBundle, blockers: &mut Vec<String>) {
    if !super::super::model::valid_content_ref(&bundle.build_cohort_ref)
        || !super::super::model::valid_content_ref(&bundle.octet_report_ref)
        || bundle.stage_receipt_refs.len() > MAX_COMPONENT_EVIDENCE_REFS
        || !super::super::model::valid_ref_collection(&bundle.stage_receipt_refs)
    {
        blockers
            .push("Mantle component bundle has missing, malformed, duplicate, or unsorted build evidence".to_string());
    }
    if !bundle.embedded_admission_refs.is_empty() {
        blockers.push("Mantle component bundle embeds circular Valence or Cairn admission evidence".to_string());
    }
}

fn validate_envelope(
    bundle: &MantleComponentBundle,
    envelope: &ComponentAdmissionEnvelope,
    blockers: &mut Vec<String>,
) {
    if envelope.schema_id != COMPONENT_ADMISSION_ENVELOPE_SCHEMA || envelope.bundle_ref != bundle.bundle_ref {
        blockers.push("component admission envelope is stale or bound to another bundle".to_string());
    }
    for (label, refs) in [
        ("Valence sidecar", &envelope.valence_sidecar_refs),
        ("Cairn acceptance", &envelope.cairn_acceptance_refs),
        ("policy", &envelope.policy_refs),
        ("authority", &envelope.authority_refs),
        ("resource", &envelope.resource_refs),
    ] {
        if refs.len() > MAX_COMPONENT_EVIDENCE_REFS || !super::super::model::valid_ref_collection(refs) {
            blockers.push(format!(
                "component admission envelope {label} refs are missing, malformed, duplicate, or unsorted"
            ));
        }
    }
}
