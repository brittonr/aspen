type Command = super::DeliveryCommand;
type Outcome<T> = molten::error::Result<T>;

pub(super) fn run(command: Command) -> Outcome<()> {
    match command {
        Command::Scope {
            scope_profile,
            scope_name,
            retention_refs,
            out,
        } => {
            let value =
                molten::delivery_idempotency::scope_profile_value(&scope_profile, &scope_name, &retention_refs)?;
            let reference = molten::preserves_rail::canonical_hash(&value)?;
            let is_written_to_file = super::io::write_optional_preserves(out.as_ref(), &value)?;
            super::io::print_or_log_summary(
                is_written_to_file,
                &format!("delivery scope ref={reference} profile={scope_profile} name={scope_name}"),
            );
            Ok(())
        }
        Command::OperationId {
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
        } => {
            let resolved_scope_ref =
                resolve_delivery_scope_ref(&scope_profile, scope_name.as_deref(), scope_ref.as_deref())?;
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
        Command::Check {
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
        } => {
            let resolved_scope_ref =
                resolve_delivery_scope_ref(&scope_profile, scope_name.as_deref(), scope_ref.as_deref())?;
            let delivery =
                molten::delivery_idempotency::check_delivery(molten::delivery_idempotency::DeliveryCheckInput {
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
            let is_written_to_file =
                super::io::write_optional_preserves(receipt_out.as_ref(), &delivery.receipt.value)?;
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
        Command::ReceiptShow { receipt_ref, root } => {
            let value = molten::delivery_idempotency::read_idempotency_receipt(&root, &receipt_ref)?;
            println!("{}", molten::delivery_idempotency::delivery_summary(&value)?);
            Ok(())
        }
        Command::Show { artifact } => {
            let value = super::io::read_preserves_file(&artifact)?;
            println!("{}", molten::delivery_idempotency::delivery_summary(&value)?);
            Ok(())
        }
    }
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
