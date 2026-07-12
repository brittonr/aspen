use super::super::model::ComponentConsumer;
use super::super::model::ComponentDenial;
use super::super::model::ComponentResult;
use super::super::model::ComponentRuntimeProfile;
use super::super::model::EvidenceScope;
use super::super::model::WasmArtifactKind;
use super::super::model::content_ref;
use super::super::model::sorted_unique;
use super::super::model::valid_content_ref;
use super::super::model::valid_ref_collection;
use super::super::profile::component_profile_ref;

pub const MANTLE_COMPONENT_BUNDLE_SCHEMA: &str = "mantle.component-materialization-bundle.v1";
pub const COMPONENT_ADMISSION_ENVELOPE_SCHEMA: &str = "molten.component-admission-envelope.v1";
const MAX_COMPONENT_EVIDENCE_REFS: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterializedObjectIdentity {
    pub content_ref: String,
    pub byte_length: u64,
}

impl MaterializedObjectIdentity {
    pub fn measure(bytes: &[u8]) -> ComponentResult<Self> {
        let byte_length = u64::try_from(bytes.len())
            .map_err(|error| ComponentDenial::new(format!("materialized object length is unsupported: {error}")))?;
        Ok(Self {
            content_ref: content_ref(bytes),
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
    pub artifact_kind: WasmArtifactKind,
    pub consumer: ComponentConsumer,
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
    pub evidence_scope: EvidenceScope,
    pub component_ref: String,
    pub wit_ref: String,
    pub bundle_ref: Option<String>,
    pub consumer: ComponentConsumer,
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
    lines.extend(sorted_unique(&bundle.stage_receipt_refs).into_iter().map(|value| format!("stage-receipt:{value}")));
    lines.extend(
        sorted_unique(&bundle.embedded_admission_refs)
            .into_iter()
            .map(|value| format!("embedded-admission:{value}")),
    );
    content_ref(lines.join("\n").as_bytes())
}

pub fn verify_materialization(
    profile: &ComponentRuntimeProfile,
    requested_scope: EvidenceScope,
    source: ComponentArtifactSource<'_>,
) -> ComponentResult<MaterializationAdmission> {
    let component = MaterializedObjectIdentity::measure(source.component_bytes())?;
    let wit = MaterializedObjectIdentity::measure(source.wit_bytes())?;
    validate_byte_bounds(profile, &component, &wit)?;
    match source {
        ComponentArtifactSource::Mantle {
            bundle,
            envelope,
            component_bytes: _,
            wit_bytes: _,
        } => verify_bundle(profile, requested_scope, bundle, envelope, &component, &wit),
        ComponentArtifactSource::TestOnlyLoose {
            component_bytes: _,
            wit_bytes: _,
        } => {
            if requested_scope != EvidenceScope::TestOnly {
                return Err(ComponentDenial::new(
                    "production component execution requires a Mantle materialization bundle",
                ));
            }
            Ok(MaterializationAdmission {
                evidence_scope: EvidenceScope::TestOnly,
                component_ref: component.content_ref,
                wit_ref: wit.content_ref,
                bundle_ref: None,
                consumer: ComponentConsumer::Actor,
                profile_ref: component_profile_ref(profile),
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
    profile: &ComponentRuntimeProfile,
    component: &MaterializedObjectIdentity,
    wit: &MaterializedObjectIdentity,
) -> ComponentResult<()> {
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
        Err(ComponentDenial::from_blockers(blockers))
    }
}

fn verify_bundle(
    profile: &ComponentRuntimeProfile,
    requested_scope: EvidenceScope,
    bundle: &MantleComponentBundle,
    envelope: &ComponentAdmissionEnvelope,
    measured_component: &MaterializedObjectIdentity,
    measured_wit: &MaterializedObjectIdentity,
) -> ComponentResult<MaterializationAdmission> {
    let mut blockers = Vec::new();
    if requested_scope != EvidenceScope::Production {
        blockers.push("Mantle production bundle must execute in production evidence scope".to_string());
    }
    if bundle.schema_id != MANTLE_COMPONENT_BUNDLE_SCHEMA {
        blockers.push("unsupported Mantle component bundle schema".to_string());
    }
    if bundle.bundle_ref != mantle_bundle_ref(bundle) {
        blockers.push("Mantle component bundle identity mismatch".to_string());
    }
    if &bundle.component != measured_component || &bundle.wit != measured_wit {
        blockers.push("Mantle component bundle object identity differs from remeasured bytes".to_string());
    }
    if bundle.artifact_kind != WasmArtifactKind::Component {
        blockers.push("Mantle component bundle is not classified as a component".to_string());
    }
    if bundle.expected_profile_id != profile.profile_id || bundle.expected_cohort_ref != component_profile_ref(profile)
    {
        blockers.push("Mantle component bundle expected profile is stale or mismatched".to_string());
    }
    if bundle.wit.content_ref != profile.wit.source_ref {
        blockers.push("Mantle component bundle WIT identity does not match the admitted profile".to_string());
    }
    validate_bundle_refs(bundle, &mut blockers);
    validate_envelope(bundle, envelope, &mut blockers);
    if !bundle.has_portable_bytes || bundle.has_precompiled_bytes {
        blockers
            .push("initial component cohort admits portable bytes and rejects precompiled deserialization".to_string());
    }
    if !blockers.is_empty() {
        return Err(ComponentDenial::from_blockers(blockers));
    }
    Ok(MaterializationAdmission {
        evidence_scope: EvidenceScope::Production,
        component_ref: measured_component.content_ref.clone(),
        wit_ref: measured_wit.content_ref.clone(),
        bundle_ref: Some(bundle.bundle_ref.clone()),
        consumer: bundle.consumer,
        profile_ref: component_profile_ref(profile),
        mantle_evidence_refs: mantle_evidence_refs(bundle),
        valence_evidence_refs: envelope.valence_sidecar_refs.clone(),
        cairn_evidence_refs: envelope.cairn_acceptance_refs.clone(),
        policy_refs: envelope.policy_refs.clone(),
        authority_refs: envelope.authority_refs.clone(),
        resource_refs: envelope.resource_refs.clone(),
    })
}

fn mantle_evidence_refs(bundle: &MantleComponentBundle) -> Vec<String> {
    let mut refs = vec![bundle.build_cohort_ref.clone(), bundle.octet_report_ref.clone()];
    refs.extend(bundle.stage_receipt_refs.clone());
    sorted_unique(&refs)
}

fn validate_bundle_refs(bundle: &MantleComponentBundle, blockers: &mut Vec<String>) {
    if !valid_content_ref(&bundle.build_cohort_ref)
        || !valid_content_ref(&bundle.octet_report_ref)
        || bundle.stage_receipt_refs.len() > MAX_COMPONENT_EVIDENCE_REFS
        || !valid_ref_collection(&bundle.stage_receipt_refs)
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
        if refs.len() > MAX_COMPONENT_EVIDENCE_REFS || !valid_ref_collection(refs) {
            blockers.push(format!(
                "component admission envelope {label} refs are missing, malformed, duplicate, or unsorted"
            ));
        }
    }
}
