mod support;

pub use support::native_identity_ref;
use support::*;

use super::*;

const MAX_PROFILE_ISSUES: usize = 16;
const MAX_EXECUTABLE_ISSUES: usize = 24;
const MAX_INGRESS_ISSUES: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeHostIssue {
    SchemaMismatch {
        field: &'static str,
        actual: String,
        expected: &'static str,
    },
    EmptyField(&'static str),
    MalformedToken {
        field: &'static str,
        value: String,
    },
    MalformedRef {
        field: &'static str,
        value: String,
    },
    DuplicateValue(&'static str),
    ZeroBound(&'static str),
    MissingNonClaim(NativeHostNonClaim),
    NotLocalLivePilot,
    ProfileMismatch(&'static str),
    IdentityMismatch(&'static str),
    StaleGeneration {
        actual: u64,
        active: u64,
    },
    CallbackBytesExceeded {
        actual: u64,
        maximum: u64,
    },
    ValueIdentityMismatch(&'static str),
    AccountedBytesMismatch {
        actual: u64,
        accounted: u64,
    },
    ByteCountOverflow(&'static str),
    MaterializedValuesRequired,
    IngressStopped,
    TooManyPortBindings {
        actual: usize,
        maximum: usize,
    },
    TooManyUnresolvedOperations {
        actual: usize,
        maximum: usize,
    },
    OperationCountOverflow,
    DuplicateOperation(String),
    OperationNotFound(String),
    OperationKindMismatch,
    InvalidOperationTransition {
        previous: NativeOperationState,
        next: NativeOperationState,
    },
    CompletionAlreadyConsumed(String),
    RemovalBlocked(Vec<String>),
}

// r[impl molten.system_extension.native_host.profile]
// r[impl molten.system_extension.native_host.nonclaims]
pub fn admit_native_host_profile(
    profile: &NativeHostProfile,
) -> Result<AdmittedNativeHostProfile, Vec<NativeHostIssue>> {
    let mut issues = Vec::with_capacity(MAX_PROFILE_ISSUES);
    if profile.schema != NATIVE_HOST_PROFILE_SCHEMA {
        issues.push(NativeHostIssue::SchemaMismatch {
            field: "profile-schema",
            actual: profile.schema.clone(),
            expected: NATIVE_HOST_PROFILE_SCHEMA,
        });
    }
    validate_token("profile-id", &profile.profile_id, &mut issues);
    for (field, value) in [
        ("profile-ref", profile.profile_ref.as_str()),
        ("execution-profile-ref", profile.execution_profile_ref.as_str()),
        ("transport-profile-ref", profile.transport_profile_ref.as_str()),
    ] {
        validate_ref(field, value, &mut issues);
    }
    if profile.alpn != NATIVE_ALPN {
        issues.push(NativeHostIssue::ProfileMismatch("alpn"));
    }
    if profile.framing != NATIVE_FRAMING {
        issues.push(NativeHostIssue::ProfileMismatch("framing"));
    }
    for (field, value) in [
        ("max-callback-input-bytes", profile.max_callback_input_bytes),
        ("max-callback-output-bytes", profile.max_callback_output_bytes),
        ("max-diagnostic-bytes", profile.max_diagnostic_bytes),
        ("max-materialized-value-bytes", profile.max_materialized_value_bytes),
    ] {
        if value == 0 {
            issues.push(NativeHostIssue::ZeroBound(field));
        }
    }
    for (field, value) in [
        ("max-instances", profile.max_instances),
        ("max-unresolved-operations", profile.max_unresolved_operations),
        ("max-port-bindings", profile.max_port_bindings),
        ("max-policy-refs", profile.max_policy_refs),
        ("max-materialized-values", profile.max_materialized_values),
    ] {
        if value == 0 {
            issues.push(NativeHostIssue::ZeroBound(field));
        }
    }
    if !profile.is_local_live_pilot {
        issues.push(NativeHostIssue::NotLocalLivePilot);
    }
    if !profile.requires_materialized_values {
        issues.push(NativeHostIssue::MaterializedValuesRequired);
    }
    if has_duplicates(&profile.non_claims) {
        issues.push(NativeHostIssue::DuplicateValue("non-claims"));
    }
    for required in REQUIRED_NATIVE_HOST_NON_CLAIMS {
        if !profile.non_claims.contains(&required) {
            issues.push(NativeHostIssue::MissingNonClaim(required));
        }
    }
    if issues.is_empty() {
        Ok(AdmittedNativeHostProfile {
            profile: profile.clone(),
        })
    } else {
        Err(issues)
    }
}

// r[impl molten.system_extension.native_host.executable]
pub fn admit_native_executable(
    profile: &AdmittedNativeHostProfile,
    evidence: &NativeExecutableEvidence,
) -> Result<AdmittedNativeExecutable, Vec<NativeHostIssue>> {
    let mut issues = Vec::with_capacity(MAX_EXECUTABLE_ISSUES);
    if evidence.schema != NATIVE_EXECUTABLE_EVIDENCE_SCHEMA {
        issues.push(NativeHostIssue::SchemaMismatch {
            field: "executable-schema",
            actual: evidence.schema.clone(),
            expected: NATIVE_EXECUTABLE_EVIDENCE_SCHEMA,
        });
    }
    for (field, value) in executable_refs(evidence) {
        validate_ref(field, value, &mut issues);
    }
    if evidence.execution_profile_ref != profile.profile.execution_profile_ref {
        issues.push(NativeHostIssue::ProfileMismatch("execution-profile-ref"));
    }
    if evidence.port_binding_refs.len() > profile.profile.max_port_bindings {
        issues.push(NativeHostIssue::TooManyPortBindings {
            actual: evidence.port_binding_refs.len(),
            maximum: profile.profile.max_port_bindings,
        });
    }
    if has_duplicates(&evidence.port_binding_refs) {
        issues.push(NativeHostIssue::DuplicateValue("port-binding-refs"));
    }
    for reference in &evidence.port_binding_refs {
        validate_ref("port-binding-ref", reference, &mut issues);
    }
    if issues.is_empty() {
        Ok(AdmittedNativeExecutable {
            executable: evidence.clone(),
            profile_ref: profile.profile.profile_ref.clone(),
        })
    } else {
        Err(issues)
    }
}

// r[impl molten.system_extension.native_host.ingress]
pub fn admit_native_ingress(
    profile: &AdmittedNativeHostProfile,
    instance: &NativeInstanceRecord,
    ingress: &NativeIngressEnvelope,
) -> Result<NativeIngressAdmission, Vec<NativeHostIssue>> {
    let mut issues = Vec::with_capacity(MAX_INGRESS_ISSUES);
    if ingress.schema != NATIVE_INGRESS_SCHEMA {
        issues.push(NativeHostIssue::SchemaMismatch {
            field: "ingress-schema",
            actual: ingress.schema.clone(),
            expected: NATIVE_INGRESS_SCHEMA,
        });
    }
    for (field, value) in ingress_refs(ingress) {
        validate_ref(field, value, &mut issues);
    }
    for (field, is_matching) in [
        ("service-id", ingress.service_id == instance.service_id),
        ("manifest-ref", ingress.manifest_ref == instance.manifest_ref),
        ("transport-profile-ref", ingress.transport_profile_ref == profile.profile.transport_profile_ref),
        ("alpn", ingress.alpn == profile.profile.alpn),
        ("framing", ingress.framing == profile.profile.framing),
    ] {
        if !is_matching {
            issues.push(NativeHostIssue::IdentityMismatch(field));
        }
    }
    if ingress.generation != instance.lifecycle.generation {
        issues.push(NativeHostIssue::StaleGeneration {
            actual: ingress.generation,
            active: instance.lifecycle.generation,
        });
    }
    let payload_bytes = u64::try_from(ingress.payload.bytes.len());
    let maximum_payload_bytes =
        profile.profile.max_callback_input_bytes.min(profile.profile.max_materialized_value_bytes);
    match payload_bytes {
        Ok(payload_bytes) => {
            if payload_bytes > maximum_payload_bytes {
                issues.push(NativeHostIssue::CallbackBytesExceeded {
                    actual: payload_bytes,
                    maximum: maximum_payload_bytes,
                });
            }
            if payload_bytes != ingress.accounted_bytes {
                issues.push(NativeHostIssue::AccountedBytesMismatch {
                    actual: payload_bytes,
                    accounted: ingress.accounted_bytes,
                });
            }
        }
        Err(_) => issues.push(NativeHostIssue::ByteCountOverflow("payload")),
    }
    let observed_payload_ref = format!("blake3:{}", blake3::hash(&ingress.payload.bytes).to_hex());
    if observed_payload_ref != ingress.payload.value_ref {
        issues.push(NativeHostIssue::ValueIdentityMismatch("payload"));
    }
    if !instance.is_accepting_ingress {
        issues.push(NativeHostIssue::IngressStopped);
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    Ok(NativeIngressAdmission {
        request_ref: ingress.request_ref.clone(),
        generation: ingress.generation,
        acknowledgement_ref: native_identity_ref(&[
            "ingress-ack-v2",
            &ingress.request_ref,
            &ingress.service_id,
            &ingress.generation.to_string(),
        ]),
    })
}
