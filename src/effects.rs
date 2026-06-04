use std::collections::BTreeSet;

use preserves::IOValue;
use preserves::Record;
use preserves::Value;

use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::EFFECT_COMPOUND_HANDLER_SCHEMA;
use crate::preserves_rail::EFFECT_DYNAMIC_OPERATION_SCHEMA;
use crate::preserves_rail::EFFECT_HANDLE_CLEANUP_SCHEMA;
use crate::preserves_rail::EFFECT_HANDLE_SCHEMA;
use crate::preserves_rail::EFFECT_HANDLER_BINDING_SCHEMA;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::u64_value;
use crate::preserves_rail::value_to_iovalue;

pub const TRANSFER_LOCAL_ONLY: &str = "local-only";
pub const TRANSFER_ATTENUATED_DELEGATION: &str = "attenuated-delegation";
pub const TRANSFER_REMOTE_PROXY: &str = "remote-proxy";

pub const ADAPTER_KIND_STORAGE: &str = "storage";
pub const ADAPTER_KIND_BLOB: &str = "blob";
pub const ADAPTER_KIND_NETWORK: &str = "network";
pub const ADAPTER_KIND_REMOTE_SYNC: &str = "remote-sync";
pub const ADAPTER_KIND_REPLAY_RECORD: &str = "replay-record";
pub const ADAPTER_KIND_HOSTCALL: &str = "hostcall";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectScope {
    pub run_ref: String,
    pub session_ref: String,
    pub actor_ref: Option<String>,
    pub turn_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandlerBindingInput {
    pub profile: String,
    pub scope: EffectScope,
    pub adapter_kind: String,
    pub adapter_ref: String,
    pub executor_preflight_ref: Option<String>,
    pub policy_ref: String,
    pub capability_context_ref: String,
    pub authority_context_ref: Option<String>,
    pub resource_refs: Vec<String>,
    pub operations: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandlerBinding {
    pub binding_ref: String,
    pub profile: String,
    pub scope: EffectScope,
    pub adapter_kind: String,
    pub adapter_ref: String,
    pub executor_preflight_ref: Option<String>,
    pub policy_ref: String,
    pub capability_context_ref: String,
    pub authority_context_ref: Option<String>,
    pub resource_refs: Vec<String>,
    pub operations: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub checks: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectHandleInput {
    pub kind: String,
    pub scope: EffectScope,
    pub handler_binding_ref: String,
    pub operations: Vec<String>,
    pub capability_context_ref: String,
    pub authority_context_ref: Option<String>,
    pub resource_refs: Vec<String>,
    pub not_before: Option<u64>,
    pub expires_at: Option<u64>,
    pub revocation_refs: Vec<String>,
    pub transfer: String,
    pub parent_handle_ref: Option<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectHandle {
    pub handle_ref: String,
    pub kind: String,
    pub scope: EffectScope,
    pub handler_binding_ref: String,
    pub operations: Vec<String>,
    pub capability_context_ref: String,
    pub authority_context_ref: Option<String>,
    pub resource_refs: Vec<String>,
    pub not_before: Option<u64>,
    pub expires_at: Option<u64>,
    pub revocation_refs: Vec<String>,
    pub transfer: String,
    pub parent_handle_ref: Option<String>,
    pub evidence_refs: Vec<String>,
    pub checks: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectHandleRequest<'a> {
    pub kind: &'a str,
    pub operation: &'a str,
    pub run_ref: &'a str,
    pub session_ref: &'a str,
    pub actor_ref: Option<&'a str>,
    pub turn_ref: Option<&'a str>,
    pub policy_ref: &'a str,
    pub capability_context_ref: &'a str,
    pub authority_context_ref: Option<&'a str>,
    pub resource_refs: &'a [String],
    pub logical_time: u64,
    pub remote_use: bool,
    pub revoked_refs: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectHandleValidation {
    pub handler_binding_ref: String,
    pub handle_ref: String,
    pub checks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompoundHandlerProfileInput {
    pub profile: String,
    pub scope: EffectScope,
    pub handler_binding_refs: Vec<String>,
    pub child_handle_refs: Vec<String>,
    pub policy_ref: String,
    pub capability_context_ref: String,
    pub authority_context_ref: Option<String>,
    pub resource_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompoundHandlerProfile {
    pub profile_ref: String,
    pub profile: String,
    pub scope: EffectScope,
    pub handler_binding_refs: Vec<String>,
    pub child_handle_refs: Vec<String>,
    pub policy_ref: String,
    pub capability_context_ref: String,
    pub authority_context_ref: Option<String>,
    pub resource_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub checks: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicOperationRecordInput {
    pub operation: String,
    pub adapter_ref: String,
    pub callable_ref: String,
    pub request_ref: String,
    pub response_ref: String,
    pub policy_ref: String,
    pub capability_context_ref: String,
    pub resource_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicOperationRecord {
    pub record_ref: String,
    pub operation: String,
    pub adapter_ref: String,
    pub callable_ref: String,
    pub request_ref: String,
    pub response_ref: String,
    pub policy_ref: String,
    pub capability_context_ref: String,
    pub resource_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub checks: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandleAttenuationInput {
    pub scope: EffectScope,
    pub operations: Vec<String>,
    pub expires_at: Option<u64>,
    pub transfer: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandleCleanupReceipt {
    pub receipt_ref: String,
    pub handle_ref: String,
    pub action: String,
    pub live_usable: bool,
    pub preserve_artifact: bool,
    pub evidence_refs: Vec<String>,
    pub checks: Vec<String>,
    pub value: IOValue,
}

pub fn handler_binding_value(input: &HandlerBindingInput) -> Result<IOValue> {
    validate_non_empty(&input.profile, "handler profile")?;
    validate_non_empty(&input.adapter_kind, "handler adapter kind")?;
    require_ref(&input.adapter_ref, "handler adapter ref")?;
    if let Some(executor_preflight_ref) = input.executor_preflight_ref.as_deref() {
        require_ref(executor_preflight_ref, "handler executor preflight ref")?;
    }
    require_ref(&input.policy_ref, "handler policy ref")?;
    require_ref(&input.capability_context_ref, "handler capability context ref")?;
    if let Some(authority_context_ref) = input.authority_context_ref.as_deref() {
        require_ref(authority_context_ref, "handler authority context ref")?;
    }
    validate_scope(&input.scope)?;
    validate_refs(&input.resource_refs, "handler resource ref")?;
    validate_operations(&input.operations)?;
    validate_refs(&input.evidence_refs, "handler evidence ref")?;
    Ok(record("handler-binding-v1", vec![
        string(EFFECT_HANDLER_BINDING_SCHEMA),
        record("profile", vec![string(&input.profile)]),
        scope_value(&input.scope),
        record("implementation", vec![
            string(&input.adapter_kind),
            string(&input.adapter_ref),
            optional_ref_value(input.executor_preflight_ref.as_deref()),
        ]),
        record("policy", vec![
            string(&input.policy_ref),
            string(&input.capability_context_ref),
            optional_ref_value(input.authority_context_ref.as_deref()),
        ]),
        refs_record("resources", &input.resource_refs),
        operations_record(&input.operations),
        refs_record("evidence", &input.evidence_refs),
        checks_value(&[
            "deny-ambient-effects",
            "handler-binding-available",
            "policy-capability-resource-binding",
            "bluefin-non-normative-prior-art",
        ]),
    ]))
}

pub fn effect_handle_value(input: &EffectHandleInput) -> Result<IOValue> {
    validate_non_empty(&input.kind, "effect handle kind")?;
    validate_scope(&input.scope)?;
    require_ref(&input.handler_binding_ref, "effect handle handler binding ref")?;
    validate_operations(&input.operations)?;
    require_ref(&input.capability_context_ref, "effect handle capability context ref")?;
    if let Some(authority_context_ref) = input.authority_context_ref.as_deref() {
        require_ref(authority_context_ref, "effect handle authority context ref")?;
    }
    validate_refs(&input.resource_refs, "effect handle resource ref")?;
    if let (Some(not_before), Some(expires_at)) = (input.not_before, input.expires_at)
        && not_before > expires_at
    {
        return Err(MoltenError::invalid_harness("effect handle validity not-before exceeds expiry"));
    }
    validate_refs(&input.revocation_refs, "effect handle revocation ref")?;
    validate_transfer(&input.transfer)?;
    if let Some(parent_handle_ref) = input.parent_handle_ref.as_deref() {
        require_ref(parent_handle_ref, "effect handle parent ref")?;
    }
    validate_refs(&input.evidence_refs, "effect handle evidence ref")?;
    Ok(record("effect-handle-v1", vec![
        string(EFFECT_HANDLE_SCHEMA),
        record("kind", vec![string(&input.kind)]),
        scope_value(&input.scope),
        record("handler", vec![string(&input.handler_binding_ref)]),
        operations_record(&input.operations),
        record("authority", vec![
            string(&input.capability_context_ref),
            optional_ref_value(input.authority_context_ref.as_deref()),
        ]),
        refs_record("resources", &input.resource_refs),
        record("validity", vec![
            optional_u64_value(input.not_before),
            optional_u64_value(input.expires_at),
            sequence(input.revocation_refs.iter().map(string).collect()),
        ]),
        record("transfer", vec![string(&input.transfer)]),
        record("parent", vec![optional_ref_value(input.parent_handle_ref.as_deref())]),
        refs_record("evidence", &input.evidence_refs),
        checks_value(&[
            "handle-not-authority",
            "handler-scoped-handle",
            "operation-set-binding",
            "scope-lifetime-binding",
        ]),
    ]))
}

pub fn parse_handler_binding(value: &IOValue) -> Result<HandlerBinding> {
    let binding = simple_record(value, "handler-binding-v1", 9)?;
    require_schema(&binding[0], EFFECT_HANDLER_BINDING_SCHEMA, "handler binding schema")?;
    let implementation = value_to_iovalue(&binding[3]);
    let implementation = simple_record(&implementation, "implementation", 3)?;
    let policy = value_to_iovalue(&binding[4]);
    let policy = simple_record(&policy, "policy", 3)?;
    let checks = parse_checks(&binding[8])?;
    require_check(&checks, "deny-ambient-effects", "handler binding")?;
    require_check(&checks, "handler-binding-available", "handler binding")?;
    Ok(HandlerBinding {
        binding_ref: canonical_hash(value)?,
        profile: required_record_string(&binding[1], "profile", "handler binding profile")?,
        scope: parse_scope(&binding[2])?,
        adapter_kind: required_string(&implementation[0], "handler adapter kind")?,
        adapter_ref: required_ref(&implementation[1], "handler adapter ref")?,
        executor_preflight_ref: parse_optional_ref_value(&implementation[2])?,
        policy_ref: required_ref(&policy[0], "handler policy ref")?,
        capability_context_ref: required_ref(&policy[1], "handler capability context ref")?,
        authority_context_ref: parse_optional_ref_value(&policy[2])?,
        resource_refs: parse_ref_sequence_record(&binding[5], "resources")?,
        operations: parse_string_sequence_record(&binding[6], "operations")?,
        evidence_refs: parse_ref_sequence_record(&binding[7], "evidence")?,
        checks,
        value: value.clone(),
    })
}

pub fn parse_effect_handle(value: &IOValue) -> Result<EffectHandle> {
    let handle = simple_record(value, "effect-handle-v1", 12)?;
    require_schema(&handle[0], EFFECT_HANDLE_SCHEMA, "effect handle schema")?;
    let authority = value_to_iovalue(&handle[5]);
    let authority = simple_record(&authority, "authority", 2)?;
    let validity = value_to_iovalue(&handle[7]);
    let validity = simple_record(&validity, "validity", 3)?;
    let revocation_values = required_sequence(&validity[2], "effect handle revocations")?;
    let mut revocation_refs = Vec::with_capacity(revocation_values.len());
    for revocation in revocation_values.iter() {
        revocation_refs.push(required_ref(revocation, "effect handle revocation ref")?);
    }
    let checks = parse_checks(&handle[11])?;
    require_check(&checks, "handle-not-authority", "effect handle")?;
    require_check(&checks, "handler-scoped-handle", "effect handle")?;
    let transfer = required_record_string(&handle[8], "transfer", "effect handle transfer")?;
    validate_transfer(&transfer)?;
    let not_before = parse_optional_u64_value(&validity[0])?;
    let expires_at = parse_optional_u64_value(&validity[1])?;
    if let (Some(not_before), Some(expires_at)) = (not_before, expires_at)
        && not_before > expires_at
    {
        return Err(MoltenError::invalid_harness("effect handle validity not-before exceeds expiry"));
    }
    Ok(EffectHandle {
        handle_ref: canonical_hash(value)?,
        kind: required_record_string(&handle[1], "kind", "effect handle kind")?,
        scope: parse_scope(&handle[2])?,
        handler_binding_ref: required_record_ref(&handle[3], "handler", "effect handle handler ref")?,
        operations: parse_string_sequence_record(&handle[4], "operations")?,
        capability_context_ref: required_ref(&authority[0], "effect handle capability context ref")?,
        authority_context_ref: parse_optional_ref_value(&authority[1])?,
        resource_refs: parse_ref_sequence_record(&handle[6], "resources")?,
        not_before,
        expires_at,
        revocation_refs,
        transfer,
        parent_handle_ref: parse_optional_ref_record(&handle[9], "parent")?,
        evidence_refs: parse_ref_sequence_record(&handle[10], "evidence")?,
        checks,
        value: value.clone(),
    })
}

pub fn validate_handle_for_request(
    handler_value: &IOValue,
    handle_value: &IOValue,
    request: &EffectHandleRequest<'_>,
) -> Result<EffectHandleValidation> {
    let handler = parse_handler_binding(handler_value)?;
    let handle = parse_effect_handle(handle_value)?;
    validate_parsed_handle_for_request(&handler, &handle, request)
}

pub fn compound_handler_profile_value(input: &CompoundHandlerProfileInput) -> Result<IOValue> {
    validate_non_empty(&input.profile, "compound handler profile")?;
    validate_scope(&input.scope)?;
    validate_refs(&input.handler_binding_refs, "compound handler binding ref")?;
    validate_refs(&input.child_handle_refs, "compound child handle ref")?;
    validate_unique_refs(&input.handler_binding_refs, "compound handler binding ref")?;
    validate_unique_refs(&input.child_handle_refs, "compound child handle ref")?;
    if input.child_handle_refs.is_empty() {
        return Err(MoltenError::invalid_harness("compound handler profile must expose at least one child handle"));
    }
    require_ref(&input.policy_ref, "compound handler policy ref")?;
    require_ref(&input.capability_context_ref, "compound handler capability context ref")?;
    if let Some(authority_context_ref) = input.authority_context_ref.as_deref() {
        require_ref(authority_context_ref, "compound handler authority context ref")?;
    }
    validate_refs(&input.resource_refs, "compound handler resource ref")?;
    validate_refs(&input.evidence_refs, "compound handler evidence ref")?;
    Ok(record("compound-handler-profile-v1", vec![
        string(EFFECT_COMPOUND_HANDLER_SCHEMA),
        record("profile", vec![string(&input.profile)]),
        scope_value(&input.scope),
        refs_record("handler-bindings", &input.handler_binding_refs),
        refs_record("child-handles", &input.child_handle_refs),
        record("policy", vec![
            string(&input.policy_ref),
            string(&input.capability_context_ref),
            optional_ref_value(input.authority_context_ref.as_deref()),
        ]),
        refs_record("resources", &input.resource_refs),
        refs_record("evidence", &input.evidence_refs),
        checks_value(&[
            "compound-handler-profile",
            "child-handle-ref-binding",
            "shared-policy-capability-resource",
            "no-ambient-effects",
        ]),
    ]))
}

pub fn parse_compound_handler_profile(value: &IOValue) -> Result<CompoundHandlerProfile> {
    let profile = simple_record(value, "compound-handler-profile-v1", 9)?;
    require_schema(&profile[0], EFFECT_COMPOUND_HANDLER_SCHEMA, "compound handler schema")?;
    let policy = value_to_iovalue(&profile[5]);
    let policy = simple_record(&policy, "policy", 3)?;
    let handler_binding_refs = parse_ref_sequence_record(&profile[3], "handler-bindings")?;
    let child_handle_refs = parse_ref_sequence_record(&profile[4], "child-handles")?;
    validate_unique_refs(&handler_binding_refs, "compound handler binding ref")?;
    validate_unique_refs(&child_handle_refs, "compound child handle ref")?;
    if child_handle_refs.is_empty() {
        return Err(MoltenError::invalid_harness("compound handler profile must expose at least one child handle"));
    }
    let checks = parse_checks(&profile[8])?;
    require_check(&checks, "compound-handler-profile", "compound handler profile")?;
    require_check(&checks, "child-handle-ref-binding", "compound handler profile")?;
    Ok(CompoundHandlerProfile {
        profile_ref: canonical_hash(value)?,
        profile: required_record_string(&profile[1], "profile", "compound handler profile")?,
        scope: parse_scope(&profile[2])?,
        handler_binding_refs,
        child_handle_refs,
        policy_ref: required_ref(&policy[0], "compound handler policy ref")?,
        capability_context_ref: required_ref(&policy[1], "compound handler capability context ref")?,
        authority_context_ref: parse_optional_ref_value(&policy[2])?,
        resource_refs: parse_ref_sequence_record(&profile[6], "resources")?,
        evidence_refs: parse_ref_sequence_record(&profile[7], "evidence")?,
        checks,
        value: value.clone(),
    })
}

pub fn dynamic_operation_record_value(input: &DynamicOperationRecordInput) -> Result<IOValue> {
    validate_operation(&input.operation)?;
    require_ref(&input.adapter_ref, "dynamic operation adapter ref")?;
    require_ref(&input.callable_ref, "dynamic operation callable ref")?;
    require_ref(&input.request_ref, "dynamic operation request ref")?;
    require_ref(&input.response_ref, "dynamic operation response ref")?;
    require_ref(&input.policy_ref, "dynamic operation policy ref")?;
    require_ref(&input.capability_context_ref, "dynamic operation capability context ref")?;
    validate_refs(&input.resource_refs, "dynamic operation resource ref")?;
    validate_refs(&input.evidence_refs, "dynamic operation evidence ref")?;
    Ok(record("dynamic-operation-v1", vec![
        string(EFFECT_DYNAMIC_OPERATION_SCHEMA),
        record("operation", vec![string(&input.operation)]),
        record("adapter", vec![string(&input.adapter_ref)]),
        record("callable", vec![string(&input.callable_ref)]),
        record("request", vec![string(&input.request_ref)]),
        record("response", vec![string(&input.response_ref)]),
        record("policy", vec![string(&input.policy_ref), string(&input.capability_context_ref)]),
        refs_record("resources", &input.resource_refs),
        refs_record("evidence", &input.evidence_refs),
        checks_value(&[
            "reviewed-dynamic-operation",
            "canonical-request-response",
            "policy-capability-resource-binding",
        ]),
    ]))
}

pub fn parse_dynamic_operation_record(value: &IOValue) -> Result<DynamicOperationRecord> {
    let operation = simple_record(value, "dynamic-operation-v1", 10)?;
    require_schema(&operation[0], EFFECT_DYNAMIC_OPERATION_SCHEMA, "dynamic operation schema")?;
    let policy = value_to_iovalue(&operation[6]);
    let policy = simple_record(&policy, "policy", 2)?;
    let checks = parse_checks(&operation[9])?;
    require_check(&checks, "reviewed-dynamic-operation", "dynamic operation")?;
    require_check(&checks, "canonical-request-response", "dynamic operation")?;
    Ok(DynamicOperationRecord {
        record_ref: canonical_hash(value)?,
        operation: required_record_string(&operation[1], "operation", "dynamic operation name")?,
        adapter_ref: required_record_ref(&operation[2], "adapter", "dynamic operation adapter ref")?,
        callable_ref: required_record_ref(&operation[3], "callable", "dynamic operation callable ref")?,
        request_ref: required_record_ref(&operation[4], "request", "dynamic operation request ref")?,
        response_ref: required_record_ref(&operation[5], "response", "dynamic operation response ref")?,
        policy_ref: required_ref(&policy[0], "dynamic operation policy ref")?,
        capability_context_ref: required_ref(&policy[1], "dynamic operation capability context ref")?,
        resource_refs: parse_ref_sequence_record(&operation[7], "resources")?,
        evidence_refs: parse_ref_sequence_record(&operation[8], "evidence")?,
        checks,
        value: value.clone(),
    })
}

pub fn attenuated_handle_value(parent_handle_value: &IOValue, input: &HandleAttenuationInput) -> Result<IOValue> {
    let parent = parse_effect_handle(parent_handle_value)?;
    validate_scope_narrows(&parent.scope, &input.scope)?;
    validate_operation_subset(&parent.operations, &input.operations)?;
    validate_transfer_attenuation(&parent.transfer, &input.transfer)?;
    if let (Some(parent_expiry), Some(child_expiry)) = (parent.expires_at, input.expires_at)
        && child_expiry > parent_expiry
    {
        return Err(MoltenError::invalid_harness("attenuated effect handle expiry exceeds parent expiry"));
    }
    if parent.expires_at.is_some() && input.expires_at.is_none() {
        return Err(MoltenError::invalid_harness("attenuated effect handle cannot remove parent expiry"));
    }
    validate_refs(&input.evidence_refs, "attenuated handle evidence ref")?;
    effect_handle_value(&EffectHandleInput {
        kind: parent.kind,
        scope: input.scope.clone(),
        handler_binding_ref: parent.handler_binding_ref,
        operations: input.operations.clone(),
        capability_context_ref: parent.capability_context_ref,
        authority_context_ref: parent.authority_context_ref,
        resource_refs: parent.resource_refs,
        not_before: parent.not_before,
        expires_at: input.expires_at.or(parent.expires_at),
        revocation_refs: parent.revocation_refs,
        transfer: input.transfer.clone(),
        parent_handle_ref: Some(parent.handle_ref),
        evidence_refs: input.evidence_refs.clone(),
    })
}

pub fn handle_cleanup_receipt_value(
    handle_ref: &str,
    action: &str,
    live_usable: bool,
    preserve_artifact: bool,
    evidence_refs: &[String],
) -> Result<IOValue> {
    require_ref(handle_ref, "cleanup handle ref")?;
    validate_non_empty(action, "cleanup action")?;
    validate_refs(evidence_refs, "cleanup evidence ref")?;
    Ok(record("handle-cleanup-v1", vec![
        string(EFFECT_HANDLE_CLEANUP_SCHEMA),
        record("handle", vec![string(handle_ref)]),
        record("action", vec![string(action)]),
        record("live-usable", vec![crate::preserves_rail::bool_value(live_usable)]),
        record("preserve-artifact", vec![crate::preserves_rail::bool_value(preserve_artifact)]),
        refs_record("evidence", evidence_refs),
        checks_value(&[
            "live-usability-cleanup",
            "historical-artifact-preserved",
            "replay-evidence-retained",
        ]),
    ]))
}

pub fn parse_handle_cleanup_receipt(value: &IOValue) -> Result<HandleCleanupReceipt> {
    let receipt = simple_record(value, "handle-cleanup-v1", 7)?;
    require_schema(&receipt[0], EFFECT_HANDLE_CLEANUP_SCHEMA, "handle cleanup schema")?;
    let checks = parse_checks(&receipt[6])?;
    require_check(&checks, "live-usability-cleanup", "handle cleanup")?;
    require_check(&checks, "historical-artifact-preserved", "handle cleanup")?;
    let should_preserve_artifact = required_record_bool(&receipt[4], "preserve-artifact", "cleanup preserve artifact")?;
    if !should_preserve_artifact {
        return Err(MoltenError::invalid_harness("handle cleanup must preserve historical artifacts for replay"));
    }
    Ok(HandleCleanupReceipt {
        receipt_ref: canonical_hash(value)?,
        handle_ref: required_record_ref(&receipt[1], "handle", "cleanup handle ref")?,
        action: required_record_string(&receipt[2], "action", "cleanup action")?,
        live_usable: required_record_bool(&receipt[3], "live-usable", "cleanup live usability")?,
        preserve_artifact: should_preserve_artifact,
        evidence_refs: parse_ref_sequence_record(&receipt[5], "evidence")?,
        checks,
        value: value.clone(),
    })
}

fn validate_parsed_handle_for_request(
    handler: &HandlerBinding,
    handle: &EffectHandle,
    request: &EffectHandleRequest<'_>,
) -> Result<EffectHandleValidation> {
    if handle.handler_binding_ref != handler.binding_ref {
        return Err(MoltenError::invalid_harness("effect handle does not bind the supplied handler binding"));
    }
    if handle.kind != request.kind {
        return Err(MoltenError::invalid_harness(format!(
            "effect handle kind mismatch: got {}, expected {}",
            handle.kind, request.kind
        )));
    }
    require_operation(&handler.operations, request.operation, "handler binding")?;
    require_operation(&handle.operations, request.operation, "effect handle")?;
    require_scope_match(&handler.scope, request, "handler binding")?;
    require_scope_match(&handle.scope, request, "effect handle")?;
    if handler.policy_ref != request.policy_ref {
        return Err(MoltenError::invalid_harness("handler binding policy ref does not match request"));
    }
    if handler.capability_context_ref != request.capability_context_ref
        || handle.capability_context_ref != request.capability_context_ref
    {
        return Err(MoltenError::invalid_harness("effect handle capability context ref does not match request"));
    }
    if handler.authority_context_ref.as_deref() != request.authority_context_ref
        || handle.authority_context_ref.as_deref() != request.authority_context_ref
    {
        return Err(MoltenError::invalid_harness("effect handle authority context ref does not match request"));
    }
    if handler.resource_refs != request.resource_refs || handle.resource_refs != request.resource_refs {
        return Err(MoltenError::invalid_harness("effect handle resource refs do not match request"));
    }
    if handle.not_before.is_some_and(|not_before| request.logical_time < not_before) {
        return Err(MoltenError::invalid_harness("effect handle used before not-before bound"));
    }
    if handle.expires_at.is_some_and(|expires_at| request.logical_time >= expires_at) {
        return Err(MoltenError::invalid_harness("effect handle expired before request"));
    }
    if request
        .revoked_refs
        .iter()
        .any(|revoked| handle.revocation_refs.iter().any(|handle_revoked| handle_revoked == revoked))
    {
        return Err(MoltenError::invalid_harness("effect handle revoked before request"));
    }
    if request.remote_use && handle.transfer == TRANSFER_LOCAL_ONLY {
        return Err(MoltenError::invalid_harness("local-only effect handle cannot be used remotely"));
    }
    if request.remote_use && handle.transfer == TRANSFER_REMOTE_PROXY {
        if handle.evidence_refs.len() < 3 {
            return Err(MoltenError::invalid_harness(
                "remote-proxy effect handle missing peer/node/revocation evidence refs",
            ));
        }
        if handle.resource_refs.is_empty() {
            return Err(MoltenError::invalid_harness("remote-proxy effect handle missing resource limits"));
        }
        if handle.expires_at.is_none() {
            return Err(MoltenError::invalid_harness("remote-proxy effect handle missing bounded expiry"));
        }
    }
    Ok(EffectHandleValidation {
        handler_binding_ref: handler.binding_ref.clone(),
        handle_ref: handle.handle_ref.clone(),
        checks: vec![
            "handler-binding-available".to_string(),
            "effect-handle-binding".to_string(),
            "handle-not-authority".to_string(),
            "operation-authorization-binding".to_string(),
            "scope-lifetime-binding".to_string(),
        ],
    })
}

fn scope_value(scope: &EffectScope) -> IOValue {
    record("scope", vec![
        record("run", vec![string(&scope.run_ref)]),
        record("session", vec![string(&scope.session_ref)]),
        record("actor", vec![optional_ref_value(scope.actor_ref.as_deref())]),
        record("turn", vec![optional_ref_value(scope.turn_ref.as_deref())]),
    ])
}

fn parse_scope(value: &Value<IOValue>) -> Result<EffectScope> {
    let value = value_to_iovalue(value);
    let scope = simple_record(&value, "scope", 4)?;
    Ok(EffectScope {
        run_ref: required_record_ref(&scope[0], "run", "effect scope run ref")?,
        session_ref: required_record_ref(&scope[1], "session", "effect scope session ref")?,
        actor_ref: parse_optional_ref_record(&scope[2], "actor")?,
        turn_ref: parse_optional_ref_record(&scope[3], "turn")?,
    })
}

fn validate_scope(scope: &EffectScope) -> Result<()> {
    require_ref(&scope.run_ref, "effect scope run ref")?;
    require_ref(&scope.session_ref, "effect scope session ref")?;
    if let Some(actor_ref) = scope.actor_ref.as_deref() {
        require_ref(actor_ref, "effect scope actor ref")?;
    }
    if let Some(turn_ref) = scope.turn_ref.as_deref() {
        require_ref(turn_ref, "effect scope turn ref")?;
    }
    Ok(())
}

fn require_scope_match(scope: &EffectScope, request: &EffectHandleRequest<'_>, label: &str) -> Result<()> {
    if scope.run_ref != request.run_ref {
        return Err(MoltenError::invalid_harness(format!("{label} run scope does not match request")));
    }
    if scope.session_ref != request.session_ref {
        return Err(MoltenError::invalid_harness(format!("{label} session scope does not match request")));
    }
    if scope.actor_ref.as_deref() != request.actor_ref {
        return Err(MoltenError::invalid_harness(format!("{label} actor scope does not match request")));
    }
    if scope.turn_ref.as_deref() != request.turn_ref {
        return Err(MoltenError::invalid_harness(format!("{label} turn scope does not match request")));
    }
    Ok(())
}

fn operations_record(operations: &[String]) -> IOValue {
    record("operations", vec![sequence(operations.iter().map(string).collect())])
}

fn refs_record(label: &'static str, refs: &[String]) -> IOValue {
    record(label, vec![sequence(refs.iter().map(string).collect())])
}

fn checks_value(checks: &[&str]) -> IOValue {
    record("checks", vec![sequence(
        checks.iter().map(|check| record("check", vec![string(*check), string("pass")])).collect(),
    )])
}

fn optional_ref_value(value: Option<&str>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn optional_u64_value(value: Option<u64>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![u64_value(value)]))
}

fn parse_optional_ref_record(value: &Value<IOValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    parse_optional_ref_value(&record[0])
}

fn parse_optional_ref_value(value: &Value<IOValue>) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(some) = value.collect_simple_record("some", Some(1)) {
        return required_ref(&some[0], "optional ref").map(Some);
    }
    required_ref(value, "optional ref").map(Some)
}

fn parse_optional_u64_value(value: &Value<IOValue>) -> Result<Option<u64>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(some) = value.collect_simple_record("some", Some(1)) {
        return required_u64(&some[0], "optional u64").map(Some);
    }
    required_u64(value, "optional u64").map(Some)
}

fn parse_ref_sequence_record(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let sequence = required_sequence(&record[0], label)?;
    let mut refs = Vec::with_capacity(sequence.len());
    for entry in sequence.iter() {
        refs.push(required_ref(entry, label)?);
    }
    Ok(refs)
}

fn parse_string_sequence_record(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let sequence = required_sequence(&record[0], label)?;
    let mut strings = Vec::with_capacity(sequence.len());
    for entry in sequence.iter() {
        strings.push(required_string(entry, label)?);
    }
    validate_operations(&strings)?;
    Ok(strings)
}

fn parse_checks(value: &Value<IOValue>) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let checks_record = simple_record(&value, "checks", 1)?;
    let check_values = required_sequence(&checks_record[0], "effect checks")?;
    let mut checks = Vec::with_capacity(check_values.len());
    for check_value in check_values.iter() {
        let check_value = value_to_iovalue(check_value);
        let check = simple_record(&check_value, "check", 2)?;
        let name = required_string(&check[0], "effect check name")?;
        let status = required_string(&check[1], "effect check status")?;
        if status != "pass" {
            return Err(MoltenError::invalid_harness(format!("effect check {name} status is {status}")));
        }
        checks.push(name);
    }
    Ok(checks)
}

fn require_check(checks: &[String], expected: &str, label: &str) -> Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} missing {expected} check")))
    }
}

fn validate_operations(operations: &[String]) -> Result<()> {
    if operations.is_empty() {
        return Err(MoltenError::invalid_harness("effect operation set must not be empty"));
    }
    let mut seen = BTreeSet::new();
    for operation in operations {
        validate_operation(operation)?;
        if !seen.insert(operation.as_str()) {
            return Err(MoltenError::invalid_harness(format!("duplicate effect operation {operation}")));
        }
    }
    Ok(())
}

fn validate_operation(operation: &str) -> Result<()> {
    validate_non_empty(operation, "effect operation")?;
    if !operation.chars().all(|character| {
        character.is_ascii_lowercase() || character.is_ascii_digit() || matches!(character, '-' | '_' | ':' | '/' | '.')
    }) {
        return Err(MoltenError::invalid_harness(format!(
            "effect operation {operation} must use lowercase ascii, digits, or effect separators"
        )));
    }
    Ok(())
}

fn validate_transfer(transfer: &str) -> Result<()> {
    match transfer {
        TRANSFER_LOCAL_ONLY | TRANSFER_ATTENUATED_DELEGATION | TRANSFER_REMOTE_PROXY => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported effect handle transfer policy {transfer}"))),
    }
}

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    for value in refs {
        require_ref(value, field)?;
    }
    Ok(())
}

fn validate_unique_refs(refs: &[String], field: &str) -> Result<()> {
    let mut seen = BTreeSet::new();
    for value in refs {
        require_ref(value, field)?;
        if !seen.insert(value.as_str()) {
            return Err(MoltenError::invalid_harness(format!("duplicate {field} {value}")));
        }
    }
    Ok(())
}

fn validate_operation_subset(parent: &[String], child: &[String]) -> Result<()> {
    validate_operations(child)?;
    for operation in child {
        if !parent.iter().any(|candidate| candidate == operation) {
            return Err(MoltenError::invalid_harness(format!(
                "attenuated effect handle operation {operation} is not in parent operation set"
            )));
        }
    }
    Ok(())
}

fn validate_scope_narrows(parent: &EffectScope, child: &EffectScope) -> Result<()> {
    validate_scope(child)?;
    if parent.run_ref != child.run_ref || parent.session_ref != child.session_ref {
        return Err(MoltenError::invalid_harness("attenuated effect handle cannot widen run/session scope"));
    }
    if let Some(parent_actor) = parent.actor_ref.as_deref()
        && child.actor_ref.as_deref() != Some(parent_actor)
    {
        return Err(MoltenError::invalid_harness("attenuated effect handle cannot escape parent actor scope"));
    }
    if let Some(parent_turn) = parent.turn_ref.as_deref()
        && child.turn_ref.as_deref() != Some(parent_turn)
    {
        return Err(MoltenError::invalid_harness("attenuated effect handle cannot escape parent turn scope"));
    }
    Ok(())
}

fn validate_transfer_attenuation(parent: &str, child: &str) -> Result<()> {
    validate_transfer(parent)?;
    validate_transfer(child)?;
    match parent {
        TRANSFER_LOCAL_ONLY if child != TRANSFER_LOCAL_ONLY => Err(MoltenError::invalid_harness(
            "attenuated effect handle cannot make a local-only parent transferable",
        )),
        TRANSFER_ATTENUATED_DELEGATION if child == TRANSFER_REMOTE_PROXY => {
            Err(MoltenError::invalid_harness("attenuated delegation handle cannot become a remote-proxy handle"))
        }
        _ => Ok(()),
    }
}

fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.is_empty() {
        Err(MoltenError::invalid_harness(format!("{field} must not be empty")))
    } else {
        Ok(())
    }
}

fn require_operation(operations: &[String], operation: &str, label: &str) -> Result<()> {
    if operations.iter().any(|candidate| candidate == operation) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} does not admit effect operation {operation}")))
    }
}

fn require_schema(value: &Value<IOValue>, expected: &str, field: &str) -> Result<()> {
    let actual = required_string(value, field)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported {field} {actual}; expected {expected}")))
    }
}

fn simple_record<'a>(
    value: &'a IOValue,
    label: &str,
    arity: usize,
) -> Result<std::borrow::Cow<'a, Record<Value<IOValue>>>> {
    value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> with arity {arity}")))
}

#[allow(clippy::owned_cow)]
fn required_sequence<'a>(value: &'a Value<IOValue>, field: &str) -> Result<std::borrow::Cow<'a, Vec<Value<IOValue>>>> {
    value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {field}")))
}

fn required_string(value: &Value<IOValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn required_u64(value: &Value<IOValue>, field: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {field}: {error}")))
}

fn required_ref(value: &Value<IOValue>, field: &str) -> Result<String> {
    let reference = required_string(value, field)?;
    require_ref(&reference, field)?;
    Ok(reference)
}

fn require_ref(value: &str, field: &str) -> Result<()> {
    if value.starts_with("blake3:") && value.len() == "blake3:".len() + 64 {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("expected blake3 hash ref for {field}, got {value}")))
    }
}

fn required_record_string(value: &Value<IOValue>, label: &str, field: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_string(&record[0], field)
}

fn required_record_bool(value: &Value<IOValue>, label: &str, field: &str) -> Result<bool> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    record[0]
        .as_boolean()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected boolean for {field}")))
}

fn required_record_ref(value: &Value<IOValue>, label: &str, field: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_ref(&record[0], field)
}

#[cfg(test)]
mod tests {
    use hegel::TestCase;
    use hegel::generators;

    use super::*;
    use crate::preserves_rail::record;
    use crate::preserves_rail::string;

    fn fake_ref(label: &str) -> String {
        canonical_hash(&record("fake-ref", vec![string(label)])).expect("hash fake ref")
    }

    fn scope(actor_ref: Option<String>) -> EffectScope {
        EffectScope {
            run_ref: fake_ref("run"),
            session_ref: fake_ref("session"),
            actor_ref,
            turn_ref: Some(fake_ref("turn")),
        }
    }

    fn binding_and_handle(operation: &str) -> (IOValue, IOValue, EffectScope, String, String) {
        let actor_ref = fake_ref("actor-a");
        let scope = scope(Some(actor_ref.clone()));
        let policy_ref = fake_ref("policy");
        let capability_ref = fake_ref("capability");
        let resource_refs = vec![fake_ref("resource")];
        let binding = handler_binding_value(&HandlerBindingInput {
            profile: "local".to_string(),
            scope: scope.clone(),
            adapter_kind: "hostcall".to_string(),
            adapter_ref: fake_ref("adapter"),
            executor_preflight_ref: Some(fake_ref("executor-preflight")),
            policy_ref: policy_ref.clone(),
            capability_context_ref: capability_ref.clone(),
            authority_context_ref: None,
            resource_refs: resource_refs.clone(),
            operations: vec![operation.to_string()],
            evidence_refs: vec![fake_ref("evidence")],
        })
        .expect("handler binding");
        let binding_ref = canonical_hash(&binding).expect("binding ref");
        let handle = effect_handle_value(&EffectHandleInput {
            kind: "hostcall".to_string(),
            scope: scope.clone(),
            handler_binding_ref: binding_ref,
            operations: vec![operation.to_string()],
            capability_context_ref: capability_ref.clone(),
            authority_context_ref: None,
            resource_refs: resource_refs.clone(),
            not_before: Some(0),
            expires_at: Some(10),
            revocation_refs: vec![fake_ref("revocation")],
            transfer: TRANSFER_LOCAL_ONLY.to_string(),
            parent_handle_ref: None,
            evidence_refs: vec![fake_ref("evidence")],
        })
        .expect("effect handle");
        (binding, handle, scope, policy_ref, capability_ref)
    }

    #[test]
    fn handle_identity_is_canonical_and_replayable() {
        let (binding, handle, scope, policy_ref, capability_ref) = binding_and_handle("send");
        let resource_refs = parse_effect_handle(&handle).expect("parse handle").resource_refs;
        let request = EffectHandleRequest {
            kind: "hostcall",
            operation: "send",
            run_ref: &scope.run_ref,
            session_ref: &scope.session_ref,
            actor_ref: scope.actor_ref.as_deref(),
            turn_ref: scope.turn_ref.as_deref(),
            policy_ref: &policy_ref,
            capability_context_ref: &capability_ref,
            authority_context_ref: None,
            resource_refs: &resource_refs,
            logical_time: 1,
            remote_use: false,
            revoked_refs: &[],
        };
        let validation = validate_handle_for_request(&binding, &handle, &request).expect("validate handle");
        assert_eq!(validation.handler_binding_ref, canonical_hash(&binding).expect("binding ref"));
        assert_eq!(validation.handle_ref, canonical_hash(&handle).expect("handle ref"));
        assert!(validation.checks.iter().any(|check| check == "handle-not-authority"));
    }

    #[test]
    fn handle_ref_alone_does_not_grant_wrong_operation() {
        let (binding, handle, scope, policy_ref, capability_ref) = binding_and_handle("send");
        let resource_refs = parse_effect_handle(&handle).expect("parse handle").resource_refs;
        let request = EffectHandleRequest {
            kind: "hostcall",
            operation: "assert",
            run_ref: &scope.run_ref,
            session_ref: &scope.session_ref,
            actor_ref: scope.actor_ref.as_deref(),
            turn_ref: scope.turn_ref.as_deref(),
            policy_ref: &policy_ref,
            capability_context_ref: &capability_ref,
            authority_context_ref: None,
            resource_refs: &resource_refs,
            logical_time: 1,
            remote_use: false,
            revoked_refs: &[],
        };
        let error = validate_handle_for_request(&binding, &handle, &request).expect_err("wrong operation denied");
        assert!(error.to_string().contains("operation"), "{error}");
    }

    #[test]
    fn same_kind_handles_in_one_scope_are_disambiguated_by_refs() {
        let scope = scope(Some(fake_ref("actor-a")));
        let policy_ref = fake_ref("policy");
        let capability_ref = fake_ref("capability");
        let resource_a = vec![fake_ref("resource-a")];
        let resource_b = vec![fake_ref("resource-b")];
        let binding_a = handler_binding_value(&HandlerBindingInput {
            profile: "local".to_string(),
            scope: scope.clone(),
            adapter_kind: "storage".to_string(),
            adapter_ref: fake_ref("storage-a"),
            executor_preflight_ref: None,
            policy_ref: policy_ref.clone(),
            capability_context_ref: capability_ref.clone(),
            authority_context_ref: None,
            resource_refs: resource_a.clone(),
            operations: vec!["read".to_string()],
            evidence_refs: vec![fake_ref("evidence-a")],
        })
        .expect("binding a");
        let binding_b = handler_binding_value(&HandlerBindingInput {
            profile: "local".to_string(),
            scope: scope.clone(),
            adapter_kind: "storage".to_string(),
            adapter_ref: fake_ref("storage-b"),
            executor_preflight_ref: None,
            policy_ref: policy_ref.clone(),
            capability_context_ref: capability_ref.clone(),
            authority_context_ref: None,
            resource_refs: resource_b.clone(),
            operations: vec!["read".to_string()],
            evidence_refs: vec![fake_ref("evidence-b")],
        })
        .expect("binding b");
        let handle_a = effect_handle_value(&EffectHandleInput {
            kind: "storage".to_string(),
            scope: scope.clone(),
            handler_binding_ref: canonical_hash(&binding_a).expect("binding a ref"),
            operations: vec!["read".to_string()],
            capability_context_ref: capability_ref.clone(),
            authority_context_ref: None,
            resource_refs: resource_a.clone(),
            not_before: None,
            expires_at: None,
            revocation_refs: Vec::new(),
            transfer: TRANSFER_LOCAL_ONLY.to_string(),
            parent_handle_ref: None,
            evidence_refs: vec![fake_ref("evidence-a")],
        })
        .expect("handle a");
        let handle_b = effect_handle_value(&EffectHandleInput {
            kind: "storage".to_string(),
            scope: scope.clone(),
            handler_binding_ref: canonical_hash(&binding_b).expect("binding b ref"),
            operations: vec!["read".to_string()],
            capability_context_ref: capability_ref.clone(),
            authority_context_ref: None,
            resource_refs: resource_b.clone(),
            not_before: None,
            expires_at: None,
            revocation_refs: Vec::new(),
            transfer: TRANSFER_LOCAL_ONLY.to_string(),
            parent_handle_ref: None,
            evidence_refs: vec![fake_ref("evidence-b")],
        })
        .expect("handle b");
        assert_ne!(canonical_hash(&handle_a).unwrap(), canonical_hash(&handle_b).unwrap());
        let request_a = EffectHandleRequest {
            kind: "storage",
            operation: "read",
            run_ref: &scope.run_ref,
            session_ref: &scope.session_ref,
            actor_ref: scope.actor_ref.as_deref(),
            turn_ref: scope.turn_ref.as_deref(),
            policy_ref: &policy_ref,
            capability_context_ref: &capability_ref,
            authority_context_ref: None,
            resource_refs: &resource_a,
            logical_time: 0,
            remote_use: false,
            revoked_refs: &[],
        };
        validate_handle_for_request(&binding_a, &handle_a, &request_a).expect("storage a handle validates");
        let error = validate_handle_for_request(&binding_b, &handle_b, &request_a)
            .expect_err("storage b cannot satisfy storage a request refs");
        assert!(error.to_string().contains("resource refs"), "{error}");
    }

    #[test]
    fn compound_dynamic_attenuation_and_cleanup_artifacts_parse() {
        let scope = scope(Some(fake_ref("actor-a")));
        let policy_ref = fake_ref("policy");
        let capability_ref = fake_ref("capability");
        let resource_refs = vec![fake_ref("resource")];
        let binding = handler_binding_value(&HandlerBindingInput {
            profile: "compound".to_string(),
            scope: scope.clone(),
            adapter_kind: ADAPTER_KIND_STORAGE.to_string(),
            adapter_ref: fake_ref("storage-adapter"),
            executor_preflight_ref: None,
            policy_ref: policy_ref.clone(),
            capability_context_ref: capability_ref.clone(),
            authority_context_ref: None,
            resource_refs: resource_refs.clone(),
            operations: vec!["read".to_string(), "write".to_string()],
            evidence_refs: vec![fake_ref("binding-evidence")],
        })
        .expect("binding");
        let parent = effect_handle_value(&EffectHandleInput {
            kind: ADAPTER_KIND_STORAGE.to_string(),
            scope: scope.clone(),
            handler_binding_ref: canonical_hash(&binding).expect("binding ref"),
            operations: vec!["read".to_string(), "write".to_string()],
            capability_context_ref: capability_ref.clone(),
            authority_context_ref: None,
            resource_refs: resource_refs.clone(),
            not_before: Some(0),
            expires_at: Some(10),
            revocation_refs: Vec::new(),
            transfer: TRANSFER_LOCAL_ONLY.to_string(),
            parent_handle_ref: None,
            evidence_refs: vec![fake_ref("parent-evidence")],
        })
        .expect("parent handle");
        let child = attenuated_handle_value(&parent, &HandleAttenuationInput {
            scope: scope.clone(),
            operations: vec!["read".to_string()],
            expires_at: Some(5),
            transfer: TRANSFER_LOCAL_ONLY.to_string(),
            evidence_refs: vec![fake_ref("attenuation-evidence")],
        })
        .expect("attenuated child");
        let profile = compound_handler_profile_value(&CompoundHandlerProfileInput {
            profile: "storage-plus-trace".to_string(),
            scope: scope.clone(),
            handler_binding_refs: vec![canonical_hash(&binding).expect("binding ref")],
            child_handle_refs: vec![
                canonical_hash(&parent).expect("parent ref"),
                canonical_hash(&child).expect("child ref"),
            ],
            policy_ref: policy_ref.clone(),
            capability_context_ref: capability_ref.clone(),
            authority_context_ref: None,
            resource_refs: resource_refs.clone(),
            evidence_refs: vec![fake_ref("profile-evidence")],
        })
        .expect("compound profile");
        let parsed_profile = parse_compound_handler_profile(&profile).expect("parse profile");
        assert_eq!(parsed_profile.child_handle_refs.len(), 2);

        let dynamic = dynamic_operation_record_value(&DynamicOperationRecordInput {
            operation: "read".to_string(),
            adapter_ref: fake_ref("storage-adapter"),
            callable_ref: fake_ref("callable"),
            request_ref: fake_ref("request"),
            response_ref: fake_ref("response"),
            policy_ref,
            capability_context_ref: capability_ref,
            resource_refs,
            evidence_refs: vec![fake_ref("dynamic-evidence")],
        })
        .expect("dynamic operation");
        assert_eq!(parse_dynamic_operation_record(&dynamic).expect("parse dynamic").operation, "read");

        let cleanup = handle_cleanup_receipt_value(
            &canonical_hash(&child).expect("child ref"),
            "revoke-live-cache",
            false,
            true,
            &[fake_ref("cleanup-evidence")],
        )
        .expect("cleanup receipt");
        let parsed_cleanup = parse_handle_cleanup_receipt(&cleanup).expect("parse cleanup");
        assert!(!parsed_cleanup.live_usable);
        assert!(parsed_cleanup.preserve_artifact);
    }

    #[test]
    fn adapter_profile_kinds_bind_storage_blob_network_remote_and_replay_handles() {
        for kind in [
            ADAPTER_KIND_STORAGE,
            ADAPTER_KIND_BLOB,
            ADAPTER_KIND_NETWORK,
            ADAPTER_KIND_REMOTE_SYNC,
            ADAPTER_KIND_REPLAY_RECORD,
        ] {
            let scope = scope(Some(fake_ref(&format!("actor-{kind}"))));
            let policy_ref = fake_ref(&format!("policy-{kind}"));
            let capability_ref = fake_ref(&format!("capability-{kind}"));
            let resource_refs = vec![fake_ref(&format!("resource-{kind}"))];
            let binding = handler_binding_value(&HandlerBindingInput {
                profile: format!("{kind}-adapter-profile"),
                scope: scope.clone(),
                adapter_kind: kind.to_string(),
                adapter_ref: fake_ref(&format!("adapter-{kind}")),
                executor_preflight_ref: None,
                policy_ref: policy_ref.clone(),
                capability_context_ref: capability_ref.clone(),
                authority_context_ref: None,
                resource_refs: resource_refs.clone(),
                operations: vec!["open".to_string()],
                evidence_refs: vec![fake_ref(&format!("evidence-{kind}"))],
            })
            .expect("adapter binding");
            let handle = effect_handle_value(&EffectHandleInput {
                kind: kind.to_string(),
                scope: scope.clone(),
                handler_binding_ref: canonical_hash(&binding).expect("binding ref"),
                operations: vec!["open".to_string()],
                capability_context_ref: capability_ref.clone(),
                authority_context_ref: None,
                resource_refs: resource_refs.clone(),
                not_before: None,
                expires_at: None,
                revocation_refs: Vec::new(),
                transfer: TRANSFER_LOCAL_ONLY.to_string(),
                parent_handle_ref: None,
                evidence_refs: vec![fake_ref(&format!("handle-evidence-{kind}"))],
            })
            .expect("adapter handle");
            validate_handle_for_request(&binding, &handle, &EffectHandleRequest {
                kind,
                operation: "open",
                run_ref: &scope.run_ref,
                session_ref: &scope.session_ref,
                actor_ref: scope.actor_ref.as_deref(),
                turn_ref: scope.turn_ref.as_deref(),
                policy_ref: &policy_ref,
                capability_context_ref: &capability_ref,
                authority_context_ref: None,
                resource_refs: &resource_refs,
                logical_time: 0,
                remote_use: false,
                revoked_refs: &[],
            })
            .expect("adapter handle validates");
        }
    }

    #[test]
    fn remote_proxy_handles_require_transfer_profile_and_explicit_refs() {
        let actor_ref = fake_ref("actor-a");
        let scope = scope(Some(actor_ref));
        let policy_ref = fake_ref("policy");
        let capability_ref = fake_ref("capability");
        let resource_refs = vec![fake_ref("resource")];
        let binding = handler_binding_value(&HandlerBindingInput {
            profile: "remote-proxy".to_string(),
            scope: scope.clone(),
            adapter_kind: ADAPTER_KIND_REMOTE_SYNC.to_string(),
            adapter_ref: fake_ref("remote-adapter"),
            executor_preflight_ref: None,
            policy_ref: policy_ref.clone(),
            capability_context_ref: capability_ref.clone(),
            authority_context_ref: None,
            resource_refs: resource_refs.clone(),
            operations: vec!["sync".to_string()],
            evidence_refs: vec![
                fake_ref("peer-agreement"),
                fake_ref("node-identity"),
                fake_ref("revocation-policy"),
            ],
        })
        .expect("remote binding");
        let handle = effect_handle_value(&EffectHandleInput {
            kind: ADAPTER_KIND_REMOTE_SYNC.to_string(),
            scope: scope.clone(),
            handler_binding_ref: canonical_hash(&binding).expect("binding ref"),
            operations: vec!["sync".to_string()],
            capability_context_ref: capability_ref.clone(),
            authority_context_ref: None,
            resource_refs: resource_refs.clone(),
            not_before: Some(0),
            expires_at: Some(5),
            revocation_refs: vec![fake_ref("remote-revocation")],
            transfer: TRANSFER_REMOTE_PROXY.to_string(),
            parent_handle_ref: None,
            evidence_refs: vec![
                fake_ref("peer-agreement"),
                fake_ref("node-identity"),
                fake_ref("revocation-policy"),
            ],
        })
        .expect("remote handle");
        validate_handle_for_request(&binding, &handle, &EffectHandleRequest {
            kind: ADAPTER_KIND_REMOTE_SYNC,
            operation: "sync",
            run_ref: &scope.run_ref,
            session_ref: &scope.session_ref,
            actor_ref: scope.actor_ref.as_deref(),
            turn_ref: scope.turn_ref.as_deref(),
            policy_ref: &policy_ref,
            capability_context_ref: &capability_ref,
            authority_context_ref: None,
            resource_refs: &resource_refs,
            logical_time: 1,
            remote_use: true,
            revoked_refs: &[],
        })
        .expect("remote proxy handle validates for remote use");
    }

    #[test]
    fn negative_security_denies_stale_revoked_wrong_scope_and_wrong_refs() {
        let (binding, handle, scope, policy_ref, capability_ref) = binding_and_handle("send");
        let parsed = parse_effect_handle(&handle).expect("parse handle");
        let base = EffectHandleRequest {
            kind: "hostcall",
            operation: "send",
            run_ref: &scope.run_ref,
            session_ref: &scope.session_ref,
            actor_ref: scope.actor_ref.as_deref(),
            turn_ref: scope.turn_ref.as_deref(),
            policy_ref: &policy_ref,
            capability_context_ref: &capability_ref,
            authority_context_ref: None,
            resource_refs: &parsed.resource_refs,
            logical_time: 1,
            remote_use: false,
            revoked_refs: &[],
        };
        validate_handle_for_request(&binding, &handle, &base).expect("base request validates");

        let stale = EffectHandleRequest {
            logical_time: 10,
            ..base.clone()
        };
        assert!(
            validate_handle_for_request(&binding, &handle, &stale)
                .expect_err("expired handle denied")
                .to_string()
                .contains("expired")
        );

        let revoked = EffectHandleRequest {
            revoked_refs: &parsed.revocation_refs,
            ..base.clone()
        };
        assert!(
            validate_handle_for_request(&binding, &handle, &revoked)
                .expect_err("revoked handle denied")
                .to_string()
                .contains("revoked")
        );

        let wrong_actor = fake_ref("other-actor");
        let wrong_scope = EffectHandleRequest {
            actor_ref: Some(&wrong_actor),
            ..base.clone()
        };
        assert!(
            validate_handle_for_request(&binding, &handle, &wrong_scope)
                .expect_err("wrong actor denied")
                .to_string()
                .contains("actor scope")
        );

        let wrong_capability = fake_ref("other-capability");
        let wrong_refs = EffectHandleRequest {
            capability_context_ref: &wrong_capability,
            ..base.clone()
        };
        assert!(
            validate_handle_for_request(&binding, &handle, &wrong_refs)
                .expect_err("wrong capability denied")
                .to_string()
                .contains("capability context")
        );
    }

    #[test]
    fn local_only_handle_denies_remote_use() {
        let (binding, handle, scope, policy_ref, capability_ref) = binding_and_handle("send");
        let resource_refs = parse_effect_handle(&handle).expect("parse handle").resource_refs;
        let request = EffectHandleRequest {
            kind: "hostcall",
            operation: "send",
            run_ref: &scope.run_ref,
            session_ref: &scope.session_ref,
            actor_ref: scope.actor_ref.as_deref(),
            turn_ref: scope.turn_ref.as_deref(),
            policy_ref: &policy_ref,
            capability_context_ref: &capability_ref,
            authority_context_ref: None,
            resource_refs: &resource_refs,
            logical_time: 1,
            remote_use: true,
            revoked_refs: &[],
        };
        let error = validate_handle_for_request(&binding, &handle, &request).expect_err("remote use denied");
        assert!(error.to_string().contains("local-only"), "{error}");
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_effect_handle_identity_attenuation_and_replay_stability(tc: TestCase) {
        let salt = tc.draw(generators::integers::<u64>().min_value(0).max_value(1_000_000));
        let is_write_operation = tc.draw(generators::booleans());
        let operation = if is_write_operation { "write" } else { "read" };
        let actor_ref = fake_ref(&format!("actor-{salt}"));
        let scope = EffectScope {
            run_ref: fake_ref(&format!("run-{salt}")),
            session_ref: fake_ref(&format!("session-{salt}")),
            actor_ref: Some(actor_ref),
            turn_ref: Some(fake_ref(&format!("turn-{salt}"))),
        };
        let policy_ref = fake_ref(&format!("policy-{salt}"));
        let capability_ref = fake_ref(&format!("capability-{salt}"));
        let resource_refs = vec![fake_ref(&format!("resource-{salt}"))];
        let binding = handler_binding_value(&HandlerBindingInput {
            profile: "property".to_string(),
            scope: scope.clone(),
            adapter_kind: ADAPTER_KIND_STORAGE.to_string(),
            adapter_ref: fake_ref(&format!("adapter-{salt}")),
            executor_preflight_ref: None,
            policy_ref: policy_ref.clone(),
            capability_context_ref: capability_ref.clone(),
            authority_context_ref: None,
            resource_refs: resource_refs.clone(),
            operations: vec!["read".to_string(), "write".to_string()],
            evidence_refs: vec![fake_ref(&format!("evidence-{salt}"))],
        })
        .expect("binding");
        let parent = effect_handle_value(&EffectHandleInput {
            kind: ADAPTER_KIND_STORAGE.to_string(),
            scope: scope.clone(),
            handler_binding_ref: canonical_hash(&binding).expect("binding ref"),
            operations: vec!["read".to_string(), "write".to_string()],
            capability_context_ref: capability_ref.clone(),
            authority_context_ref: None,
            resource_refs: resource_refs.clone(),
            not_before: Some(0),
            expires_at: Some(10),
            revocation_refs: Vec::new(),
            transfer: TRANSFER_LOCAL_ONLY.to_string(),
            parent_handle_ref: None,
            evidence_refs: vec![fake_ref(&format!("parent-evidence-{salt}"))],
        })
        .expect("parent");
        let repeated_parent = effect_handle_value(&EffectHandleInput {
            kind: ADAPTER_KIND_STORAGE.to_string(),
            scope: scope.clone(),
            handler_binding_ref: canonical_hash(&binding).expect("binding ref again"),
            operations: vec!["read".to_string(), "write".to_string()],
            capability_context_ref: capability_ref.clone(),
            authority_context_ref: None,
            resource_refs: resource_refs.clone(),
            not_before: Some(0),
            expires_at: Some(10),
            revocation_refs: Vec::new(),
            transfer: TRANSFER_LOCAL_ONLY.to_string(),
            parent_handle_ref: None,
            evidence_refs: vec![fake_ref(&format!("parent-evidence-{salt}"))],
        })
        .expect("repeated parent");
        assert_eq!(canonical_hash(&parent).unwrap(), canonical_hash(&repeated_parent).unwrap());
        let child = attenuated_handle_value(&parent, &HandleAttenuationInput {
            scope: scope.clone(),
            operations: vec![operation.to_string()],
            expires_at: Some(5),
            transfer: TRANSFER_LOCAL_ONLY.to_string(),
            evidence_refs: vec![fake_ref(&format!("attenuation-{salt}"))],
        })
        .expect("attenuated child");
        validate_handle_for_request(&binding, &child, &EffectHandleRequest {
            kind: ADAPTER_KIND_STORAGE,
            operation,
            run_ref: &scope.run_ref,
            session_ref: &scope.session_ref,
            actor_ref: scope.actor_ref.as_deref(),
            turn_ref: scope.turn_ref.as_deref(),
            policy_ref: &policy_ref,
            capability_context_ref: &capability_ref,
            authority_context_ref: None,
            resource_refs: &resource_refs,
            logical_time: 1,
            remote_use: false,
            revoked_refs: &[],
        })
        .expect("attenuated child validates");
        let other = if is_write_operation { "read" } else { "write" };
        assert!(
            validate_handle_for_request(&binding, &child, &EffectHandleRequest {
                kind: ADAPTER_KIND_STORAGE,
                operation: other,
                run_ref: &scope.run_ref,
                session_ref: &scope.session_ref,
                actor_ref: scope.actor_ref.as_deref(),
                turn_ref: scope.turn_ref.as_deref(),
                policy_ref: &policy_ref,
                capability_context_ref: &capability_ref,
                authority_context_ref: None,
                resource_refs: &resource_refs,
                logical_time: 1,
                remote_use: false,
                revoked_refs: &[],
            })
            .is_err()
        );
    }
}
