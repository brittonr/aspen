//! Read actual cached completion and freshly authorized proposal values after owner shutdown.

use preserves::ValueImpl;

pub(super) fn materialize(
    fixture: &crate::producer::Node,
    native: &molten::system_extension::NativeInstanceRecord,
    effect: &str,
) -> crate::producer::Result<(preserves::IOValue, preserves::IOValue)> {
    let mut effects = native.completed_operations.iter().filter(|operation| {
        operation.kind == molten::system_extension::NativeOperationKind::Effect && operation.parent_ref == effect
    });
    let completed = effects.next().ok_or("actual terminal effect missing")?;
    if effects.next().is_some()
        || completed.state != molten::system_extension::NativeOperationState::Terminal
        || !native.completed_operation_refs.contains(&completed.operation_ref)
    {
        return Err("the actual effect is duplicated, nonterminal or lacks consumed completion".into());
    }
    let terminal = completed.terminal_ref.as_deref().ok_or("actual effect output missing")?;
    let value = find_completion(fixture, native, effect, terminal)?;
    let fields = molten::preserves_rail::simple_record_fields(&value, "system-extension-effect-completion-v2", 11)?;
    let output = inline(&molten::preserves_rail::value_to_iovalue(&fields[9]), terminal)?;
    Ok((output, value))
}

pub(in crate::driver) fn find_completion(
    fixture: &crate::producer::Node,
    native: &molten::system_extension::NativeInstanceRecord,
    effect: &str,
    output: &str,
) -> crate::producer::Result<preserves::IOValue> {
    let values = fixture
        .root
        .ledger()?
        .open_subdir(&molten::node_state::RelativePath::parse(&fixture.runtime.callback_namespace)?)?
        .open_subdir(&molten::node_state::RelativePath::parse("values")?)?;
    let entries = values.list_entries()?;
    if entries.len() > 1024 {
        return Err("actual callback cache exceeds the fixture observation bound".into());
    }
    let mut found = None;
    for entry in entries {
        // Only regular immutable values are candidates. Failed publication may leave a blocked leaf;
        // this observer neither follows it nor treats the surrounding cache as an integrity proof.
        if entry.kind != molten::node_state::EntryKind::RegularFile {
            continue;
        }
        let Some(digest) = entry.name.strip_suffix(".preserves") else {
            continue;
        };
        let reference = format!("blake3:{digest}");
        molten::preserves_rail::validate_content_ref(&reference)?;
        let bytes = values.read(&entry.path, molten::collaboration_scope::MAX_RECORD_BYTES)?;
        if molten::preserves_rail::content_ref_from_bytes(&bytes) != reference {
            return Err("actual cached value identity mismatched its immutable address".into());
        }
        // Callback values are opaque bytes; only canonical records can be completion candidates.
        let Ok(value) = molten::preserves_rail::parse_canonical_bytes(&bytes) else {
            continue;
        };
        if matches_completion(&value, native, effect, output)? && found.replace(value).is_some() {
            return Err("native effect has multiple matching cached completions".into());
        }
    }
    found.ok_or_else(|| "actual cached completion for the terminal native effect is absent".into())
}

fn matches_completion(
    value: &preserves::IOValue,
    native: &molten::system_extension::NativeInstanceRecord,
    effect: &str,
    output: &str,
) -> crate::producer::Result<bool> {
    let Some(fields) = value.collect_simple_record("system-extension-effect-completion-v2", Some(11)) else {
        return Ok(false);
    };
    if molten::preserves_rail::record_string_field(&fields[5], "request-ref", "completion request")? != effect
        || molten::preserves_rail::record_string_field(&fields[8], "output-ref", "completion output")? != output
    {
        return Ok(false);
    }
    if molten::preserves_rail::required_string_field(&fields[0], "completion schema")?
        != molten::system_extension::SYSTEM_EXTENSION_EFFECT_COMPLETION_SCHEMA
    {
        return Err("actual cached completion schema differs".into());
    }
    let binding = molten::preserves_rail::record_string_field(&fields[2], "binding-ref", "completion binding")?;
    let generation = fields[6].collect_simple_record("generation", Some(1)).ok_or("completion generation")?;
    let generation = crate::driver::operations::wire::number(&generation[0])?;
    let operation_ref = molten_core::system_extension::native_identity_ref(&[
        "native-effect-operation-v2",
        effect,
        &binding,
        &generation.to_string(),
    ]);
    if !native.completed_operations.iter().any(|operation| {
        operation.kind == molten::system_extension::NativeOperationKind::Effect
            && operation.operation_ref == operation_ref
            && operation.parent_ref == effect
            && operation.generation == generation
            && operation.terminal_ref.as_deref() == Some(output)
            && operation.state == molten::system_extension::NativeOperationState::Terminal
    }) {
        return Ok(false);
    }
    let receipt = molten::preserves_rail::record_string_field(&fields[1], "callback-receipt-ref", "request callback")?;
    if !native.completed_operations.iter().any(|operation| {
        operation.kind == molten::system_extension::NativeOperationKind::Ingress
            && operation.parent_ref == effect
            && operation.terminal_ref.as_deref() == Some(receipt.as_str())
            && operation.state == molten::system_extension::NativeOperationState::Terminal
    }) {
        return Err("actual completion does not link the exact native Request callback".into());
    }
    Ok(true)
}

pub(in crate::driver) fn inline(
    value: &preserves::IOValue,
    output_ref: &str,
) -> crate::producer::Result<preserves::IOValue> {
    let fields = molten::preserves_rail::simple_record_fields(value, "output-value", 1)?;
    let optional = molten::preserves_rail::value_to_iovalue(&fields[0]);
    let fields = molten::preserves_rail::simple_record_fields(&optional, "some", 1)?;
    let materialized = molten::preserves_rail::value_to_iovalue(&fields[0]);
    let fields = molten::preserves_rail::simple_record_fields(&materialized, "native-callback-value-v2", 2)?;
    if molten::preserves_rail::required_string_field(&fields[0], "inline output ref")? != output_ref {
        return Err("inline completion output reference differs from native terminal output".into());
    }
    let values = molten::preserves_rail::required_sequence_field(&fields[1], "inline output bytes")?;
    if u64::try_from(values.len())? > molten::collaboration_scope::MAX_RECORD_BYTES {
        return Err("inline provider output exceeds the admitted bound".into());
    }
    let bytes = values
        .as_slice()
        .iter()
        .map(|value| Ok(u8::try_from(crate::driver::operations::wire::number(value)?)?))
        .collect::<crate::producer::Result<Vec<_>>>()?;
    if molten::preserves_rail::content_ref_from_bytes(&bytes) != output_ref {
        return Err("inline provider output bytes differ from the native terminal identity".into());
    }
    Ok(molten::preserves_rail::parse_canonical_bytes(&bytes)?)
}

fn cached(fixture: &crate::producer::Node, reference: &str) -> crate::producer::Result<preserves::IOValue> {
    let namespace = fixture
        .root
        .ledger()?
        .open_subdir(&molten::node_state::RelativePath::parse(&fixture.runtime.callback_namespace)?)?;
    let mut values = molten::system_extension::CapabilityCallbackValuePort::new(namespace)?;
    let value = molten::system_extension::NativeCallbackValuePort::materialize(
        &mut values,
        reference,
        molten::collaboration_scope::MAX_RECORD_BYTES,
    )
    .map_err(|error| format!("actual cached value unavailable: {error:?}"))?;
    Ok(molten::preserves_rail::parse_canonical_bytes(&value.bytes)?)
}

pub(in crate::driver) fn require(
    fixture: &crate::producer::Node,
    intent: &crate::driver::operations::Intent,
    committed: &preserves::IOValue,
    state: &molten::collaboration_scope::State,
) -> crate::producer::Result<preserves::IOValue> {
    let fields = molten::preserves_rail::simple_record_fields(committed, "collaboration-committed-command-v1", 8)?;
    for (index, expected) in [
        (0, intent.prepared.effect_ref.as_str()),
        (2, intent.prepared.request.reference.as_str()),
    ] {
        if molten::preserves_rail::required_string_field(&fields[index], "committed command identity")? != expected {
            return Err("actual committed command differs from the intended request".into());
        }
    }
    let proposal = require_proposal(fixture, intent, &crate::driver::operations::wire::text(&fields[1])?)?;
    let state_ref = molten::collaboration_scope::validated_state_ref(state)?;
    if molten::preserves_rail::required_string_field(&fields[5], "committed state")? != state_ref.as_str()
        || molten::preserves_rail::canonical_hash(&molten::preserves_rail::value_to_iovalue(&fields[4]))?
            != molten::preserves_rail::required_string_field(&fields[3], "application receipt")?
    {
        return Err("committed result differs from the actual durable application".into());
    }
    let position = molten::preserves_rail::value_to_iovalue(&fields[6]);
    let position = molten::preserves_rail::simple_record_fields(&position, "collaboration-live-commit-position-v1", 7)?;
    let index = crate::driver::operations::wire::number(&position[2])?;
    if index == 0
        || crate::driver::operations::wire::number(&position[3])? < index
        || crate::driver::operations::wire::number(&position[4])? < index
        || state.currentness.committed_index < index
        || crate::driver::operations::wire::text(&position[0])? != fixture.runtime.replica.group_binding_ref
        || crate::driver::operations::wire::number(&position[1])? != fixture.runtime.group.service_generation
    {
        return Err("the actual Raft position does not establish this applied command".into());
    }
    for field in [5, 6] {
        molten::preserves_rail::validate_content_ref(&crate::driver::operations::wire::text(&position[field])?)?;
    }
    Ok(proposal)
}

pub(in crate::driver) fn require_proposal(
    fixture: &crate::producer::Node,
    intent: &crate::driver::operations::Intent,
    reference: &str,
) -> crate::producer::Result<preserves::IOValue> {
    let value = cached(fixture, reference)?;
    let fields = molten::preserves_rail::simple_record_fields(&value, "collaboration-command-v1", 2)?;
    let request =
        molten::collaboration_scope::parse_mutation_request(&molten::preserves_rail::value_to_iovalue(&fields[0]))?;
    if request != intent.prepared.request {
        return Err("freshly authorized proposal substituted the exact intended mutation".into());
    }
    let decisions = molten::preserves_rail::required_sequence_field(&fields[1], "actual scope decisions")?;
    if decisions.len() != 1 {
        return Err("actual create proposal must carry the one required scope policy decision".into());
    }
    let decision =
        molten::collaboration_scope::parse_decision(&molten::preserves_rail::value_to_iovalue(&decisions[0]))?;
    if !decision.is_allowed
        || decision.request != request.reference
        || decision.subject != request.actor
        || decision.currentness != request.currentness
        || decision.policy != molten::collaboration_scope::policy_ref(crate::driver::setup::SCOPE_POLICY)?
        || decision.ability != molten::collaboration_scope::Ability::Create
    {
        return Err("actual proposal lacks its exact allowed current create-scope decision".into());
    }
    Ok(value)
}
