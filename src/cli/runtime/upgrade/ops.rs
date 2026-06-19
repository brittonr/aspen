type Command = super::UpgradeCommand;
type Outcome<T> = molten::error::Result<T>;

pub(super) fn run(command: Command) -> Outcome<()> {
    match command {
        Command::PlanNameMove {
            ledger,
            registry,
            session_id,
            name,
            from_ref,
            to_ref,
            source_gate_receipts,
            out,
        } => {
            if source_gate_receipts.is_empty() {
                return Err(molten::error::MoltenError::invalid_harness(
                    "upgrade plan-name-move requires --source-gate-receipt for strict Octet source-gate validation",
                ));
            }
            let source_gate_receipt_values = source_gate_receipts
                .iter()
                .map(|path| super::io::read_preserves_file(path))
                .collect::<Outcome<Vec<_>>>()?;
            let plan = molten::upgrades::name_move_plan_value_with_registry(
                registry.as_deref(),
                &ledger,
                &molten::upgrades::NameMovePlanInput {
                    session_id,
                    name: name.clone(),
                    from_ref,
                    to_ref,
                    initiator_ref: cli_upgrade_ref("initiator", &name)?,
                    capability_refs: vec![cli_upgrade_ref("capability", &name)?],
                    policy_refs: vec![cli_upgrade_ref("policy", &name)?],
                    evidence_refs: vec![cli_upgrade_ref("transcript", &name)?],
                    source_gate_receipt_values,
                },
            )?;
            let plan_ref = molten::preserves_rail::canonical_hash(&plan)?;
            super::io::write_file(&out, &molten::preserves_rail::to_text(&plan)?)?;
            println!("upgrade plan-name-move ok plan={} out={}", plan_ref, out.display());
            Ok(())
        }
        Command::Create {
            plan,
            store,
            receipt_out,
        } => {
            let plan_value = super::io::read_preserves_file(&plan)?;
            let created = molten::upgrades::create_session(&store, &plan_value)?;
            super::io::emit_named_receipt(receipt_out.as_ref(), "upgrade receipt", &created.receipt.value)?;
            println!(
                "upgrade create ok session={} plan={} tasks={} store={}",
                created.plan.session_id,
                created.plan.plan_ref,
                created.plan.tasks.len(),
                store.display()
            );
            Ok(())
        }
        Command::SetName {
            store,
            name,
            artifact_ref,
            receipt_out,
        } => {
            let receipt = molten::upgrades::set_name_pointer(&store, &name, &artifact_ref)?;
            super::io::emit_named_receipt(receipt_out.as_ref(), "upgrade receipt", &receipt.value)?;
            println!("upgrade set-name ok name={} artifact={} store={}", name, artifact_ref, store.display());
            Ok(())
        }
        Command::RunTask {
            store,
            ledger,
            plan_ref,
            task_id,
            receipt_out,
        } => {
            let executed = molten::upgrades::execute_task(&store, &ledger, &plan_ref, &task_id)?;
            super::io::emit_named_receipt(receipt_out.as_ref(), "upgrade receipt", &executed.receipt.value)?;
            println!(
                "upgrade run-task ok plan={} task={} kind={} decision={}",
                executed.plan_ref, executed.task_id, executed.task_kind, executed.receipt.decision
            );
            Ok(())
        }
        Command::Rollback {
            store,
            plan_ref,
            task_id,
            receipt_out,
        } => {
            let receipt = molten::upgrades::rollback_task(&store, &plan_ref, &task_id)?;
            super::io::emit_named_receipt(receipt_out.as_ref(), "upgrade receipt", &receipt.value)?;
            println!(
                "upgrade rollback {} plan={} task={} receipt={}",
                receipt.decision, plan_ref, task_id, receipt.receipt_ref
            );
            Ok(())
        }
        Command::Status { store, plan_ref } => {
            let status = molten::upgrades::status(&store, &plan_ref)?;
            println!(
                "upgrade status session={} plan={} remaining={}",
                status.session_id,
                status.plan_ref,
                status.remaining_task_ids.len()
            );
            for task in status.tasks {
                println!(
                    "{} {} {} {}",
                    task.task_id,
                    task.kind,
                    if task.done { "done" } else { "todo" },
                    task.receipt_ref.unwrap_or_else(|| "-".to_string())
                );
            }
            Ok(())
        }
        Command::CleanupCheck {
            store,
            ledger,
            registry,
            artifact_ref,
            receipt_out,
        } => {
            let receipt =
                molten::upgrades::cleanup_admission_with_registry(&store, &ledger, registry.as_deref(), &artifact_ref)?;
            super::io::emit_named_receipt(receipt_out.as_ref(), "upgrade receipt", &receipt.value)?;
            println!(
                "upgrade cleanup-check {} artifact={} receipt={}",
                receipt.decision, artifact_ref, receipt.receipt_ref
            );
            Ok(())
        }
    }
}

fn cli_upgrade_ref(kind: &str, label: &str) -> Outcome<String> {
    molten::preserves_rail::canonical_hash(&molten::preserves_rail::record("upgrade-cli-ref", vec![
        molten::preserves_rail::string(kind),
        molten::preserves_rail::string(label),
    ]))
}
