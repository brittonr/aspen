type Command = super::DeliveryCommand;
type FilePath = std::path::PathBuf;
type Outcome<T> = molten::error::Result<T>;

pub(super) fn run(command: Command) -> Outcome<()> {
    match command {
        Command::Scope {
            scope_profile,
            scope_name,
            retention_refs,
            out,
        } => scope(scope_profile, scope_name, retention_refs, out),
        command @ Command::OperationId { .. } => operation_id(command),
        command @ Command::Check { .. } => check(command),
        Command::ReceiptShow { receipt_ref, root } => receipt_show(receipt_ref, root),
        Command::Show { artifact } => show(artifact),
    }
}

fn scope(scope_profile: String, scope_name: String, retention_refs: Vec<String>, out: Option<FilePath>) -> Outcome<()> {
    let value = molten::delivery_idempotency::scope_profile_value(&scope_profile, &scope_name, &retention_refs)?;
    let reference = molten::preserves_rail::canonical_hash(&value)?;
    let is_written_to_file = super::io::write_optional_preserves(out.as_ref(), &value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!("delivery scope ref={reference} profile={scope_profile} name={scope_name}"),
    );
    Ok(())
}

fn operation_id(command: Command) -> Outcome<()> {
    let Command::OperationId {
        scope_profile,
        scope_name,
        scope_ref,
        producer,
        consumer,
        sequence,
        intent,
        payload_ref,
        policy_refs,
        out,
    } = command
    else {
        return Err(wrong_handler("operation"));
    };
    let resolved_scope_ref = resolve_delivery_scope_ref(&scope_profile, scope_name.as_deref(), scope_ref.as_deref())?;
    let operation =
        molten::delivery_idempotency::derive_operation_id(molten::delivery_idempotency::OperationIdInput {
            scope_ref: resolved_scope_ref,
            producer,
            consumer,
            sequence,
            intent,
            payload_ref,
            policy_refs,
        })?;
    let is_written_to_file = super::io::write_optional_preserves(out.as_ref(), &operation.value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!(
            "delivery operation ref={} scope={} sequence={} intent={}",
            operation.operation_ref, operation.scope_ref, operation.sequence, operation.intent
        ),
    );
    Ok(())
}

fn check(command: Command) -> Outcome<()> {
    let Command::Check {
        root,
        scope_profile,
        scope_name,
        scope_ref,
        producer,
        consumer,
        sequence,
        intent,
        payload_ref,
        policy_refs,
        evidence_refs,
        semantic_result_ref,
        gap_policy,
        receipt_out,
    } = command
    else {
        return Err(wrong_handler("check"));
    };
    let resolved_scope_ref = resolve_delivery_scope_ref(&scope_profile, scope_name.as_deref(), scope_ref.as_deref())?;
    let delivery = molten::delivery_idempotency::check(molten::delivery_idempotency::CheckInput {
        root: &root,
        scope_profile: &scope_profile,
        scope_ref: &resolved_scope_ref,
        producer: &producer,
        consumer: &consumer,
        sequence,
        intent: &intent,
        payload_ref: &payload_ref,
        policy_refs: &policy_refs,
        evidence_refs: &evidence_refs,
        semantic_result_ref: semantic_result_ref.as_deref(),
        gap_policy: parse_delivery_gap_policy(&gap_policy)?,
    })?;
    let is_written_to_file = super::io::write_optional_preserves(receipt_out.as_ref(), &delivery.receipt.value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!(
            "delivery idempotency decision={} operation={} receipt={} side_effect={} prior={}",
            delivery.receipt.decision,
            delivery.operation.operation_ref,
            delivery.receipt.receipt_ref,
            delivery.receipt.side_effect,
            delivery.prior_semantic_result_ref.as_deref().unwrap_or("none")
        ),
    );
    Ok(())
}

fn receipt_show(receipt_ref: String, root: FilePath) -> Outcome<()> {
    let value = molten::delivery_idempotency::read_idempotency_receipt(&root, &receipt_ref)?;
    println!("{}", molten::delivery_idempotency::summary(&value)?);
    Ok(())
}

fn show(artifact: FilePath) -> Outcome<()> {
    let value = super::io::read_preserves_file(&artifact)?;
    println!("{}", molten::delivery_idempotency::summary(&value)?);
    Ok(())
}

fn resolve_delivery_scope_ref(
    scope_profile: &str,
    scope_name: Option<&str>,
    scope_ref: Option<&str>,
) -> Outcome<String> {
    match (scope_name, scope_ref) {
        (_, Some(reference)) => Ok(reference.to_string()),
        (Some(name), None) => molten::delivery_idempotency::scope_ref(scope_profile, name),
        (None, None) => {
            Err(molten::error::MoltenError::invalid_harness("delivery command requires --scope-ref or --scope-name"))
        }
    }
}

fn parse_delivery_gap_policy(value: &str) -> Outcome<molten::delivery_idempotency::GapPolicy> {
    match value {
        "deny" => Ok(molten::delivery_idempotency::GapPolicy::Deny),
        "retry" => Ok(molten::delivery_idempotency::GapPolicy::Retry),
        other => Err(molten::error::MoltenError::invalid_harness(format!(
            "unsupported delivery gap policy {other}; expected deny or retry"
        ))),
    }
}

fn wrong_handler(name: &str) -> molten::error::MoltenError {
    molten::error::MoltenError::invalid_harness(format!("delivery {name} handler called with another command"))
}
