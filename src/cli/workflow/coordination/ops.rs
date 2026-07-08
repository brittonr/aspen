type Command = super::CoordinationCommand;
type FilePath = std::path::PathBuf;
type Outcome<T> = molten::error::Result<T>;

const COORDINATION_CLI_BATCH_REF_LIMIT: usize = 4096;
const COORDINATION_CLI_BATCH_EVIDENCE_LIMIT: usize = 16384;
const _: () = assert!(COORDINATION_CLI_BATCH_REF_LIMIT <= 100_000);
const _: () = assert!(COORDINATION_CLI_BATCH_EVIDENCE_LIMIT <= 100_000);

pub(super) fn run(command: Command) -> Outcome<()> {
    match command {
        command @ Command::Manifest { .. } => manifest(command),
        command @ Command::Request { .. } => request(command),
        Command::Apply {
            manifest,
            requests,
            out,
        } => run_apply(manifest, requests, out),
        Command::RunFixture { out } => run_fixture(out),
        Command::Show { artifact } => show(artifact),
    }
}

fn manifest(command: Command) -> Outcome<()> {
    let Command::Manifest {
        service_id,
        services,
        control_group_ref,
        queue_capacity,
        semaphore_capacity,
        rate_limit,
        barrier_parties,
        policy_refs,
        resource_refs,
        out,
    } = command
    else {
        return Err(wrong_handler("manifest"));
    };
    let control_group_ref = match control_group_ref {
        Some(reference) => reference,
        None => molten::preserves_rail::canonical_hash(
            &molten::raft_control_plane::control_registry_fixture_manifest_value()?,
        )?,
    };
    let services = if services.is_empty() {
        molten::coordination::coordination_supported_services()
    } else {
        services
    };
    let value = molten::coordination::coordination_service_manifest_value(
        &molten::coordination::CoordinationServiceManifestInput {
            service_id,
            services,
            control_group_ref,
            queue_capacity,
            semaphore_capacity,
            rate_limit,
            barrier_parties,
            policy_refs,
            resource_refs,
        },
    )?;
    let reference = molten::preserves_rail::canonical_hash(&value)?;
    let is_written_to_file = super::io::write_optional_preserves(out.as_ref(), &value)?;
    super::io::print_or_log_summary(is_written_to_file, &format!("coordination manifest ref={reference}"));
    Ok(())
}

fn request(command: Command) -> Outcome<()> {
    let Command::Request {
        service,
        operation,
        key,
        client_session,
        operation_id_ref,
        read_consistency_mode,
        payload,
        authority_refs,
        resource_refs,
        policy_refs,
        out,
    } = command
    else {
        return Err(wrong_handler("request"));
    };
    let payload = payload.as_ref().map(|path| super::io::read_preserves_file(path)).transpose()?;
    let value = molten::coordination::coordination_request_value(&molten::coordination::CoordinationRequestInput {
        service,
        operation,
        key,
        client_session,
        operation_id_ref,
        read_consistency_mode,
        payload,
        authority_refs,
        resource_refs,
        policy_refs,
    })?;
    let reference = molten::preserves_rail::canonical_hash(&value)?;
    let is_written_to_file = super::io::write_optional_preserves(out.as_ref(), &value)?;
    super::io::print_or_log_summary(is_written_to_file, &format!("coordination request ref={reference}"));
    Ok(())
}

fn run_fixture(out: FilePath) -> Outcome<()> {
    let run = molten::coordination::run_coordination_fixture()?;
    std::fs::create_dir_all(&out).map_err(molten::error::MoltenError::from)?;
    super::io::write_file(&out.join("report.preserves"), &molten::preserves_rail::to_text(&run.report_value)?)?;
    super::io::write_indexed_values(&out, "evidence", &run.evidence_values)?;
    println!(
        "coordination fixture decision={} manifest={} state={} receipts={} assertions={} out={}",
        run.decision,
        run.manifest_ref,
        run.final_state_ref,
        run.receipt_refs.len(),
        run.assertion_refs.len(),
        out.display()
    );
    Ok(())
}

fn show(artifact: FilePath) -> Outcome<()> {
    let value = super::io::read_preserves_file(&artifact)?;
    println!("{}", molten::coordination::coordination_summary(&value)?);
    Ok(())
}

fn run_apply(manifest: FilePath, requests: Vec<FilePath>, out: FilePath) -> Outcome<()> {
    if requests.is_empty() {
        return Err(molten::error::MoltenError::invalid_harness(
            "coordination apply requires at least one --request file",
        ));
    }
    let manifest_value = super::io::read_preserves_file(&manifest)?;
    let mut runtime = molten::coordination::new_coordination_runtime(&manifest_value)?;
    let manifest_ref = runtime.manifest.manifest_ref.clone();
    let mut decision = "pass";
    let mut evidence_values =
        super::bounded::BoundedItems::new(COORDINATION_CLI_BATCH_EVIDENCE_LIMIT, "coordination apply evidence");
    evidence_values.push(manifest_value)?;
    let mut receipt_refs =
        super::bounded::BoundedItems::new(COORDINATION_CLI_BATCH_REF_LIMIT, "coordination apply receipts");
    let mut assertion_refs =
        super::bounded::BoundedItems::new(COORDINATION_CLI_BATCH_REF_LIMIT, "coordination apply assertions");
    for request in requests {
        let request_value = super::io::read_preserves_file(&request)?;
        let result = molten::coordination::apply_coordination_request(&mut runtime, &request_value)?;
        if result.receipt.decision != "pass" {
            decision = "deny";
        }
        receipt_refs.push(result.receipt.receipt_ref.clone())?;
        for assertion in &result.assertions {
            assertion_refs.push(assertion.assertion_ref.clone())?;
        }
        for value in &result.evidence_values {
            evidence_values.push(value.clone())?;
        }
    }
    let final_state_value = molten::coordination::coordination_state_snapshot_value(&runtime.state)?;
    let final_state_ref = molten::preserves_rail::canonical_hash(&final_state_value)?;
    evidence_values.push(final_state_value)?;
    let evidence_values = evidence_values.into_vec();
    let receipt_refs = receipt_refs.into_vec();
    let assertion_refs = assertion_refs.into_vec();
    let evidence_refs =
        evidence_values.iter().map(molten::preserves_rail::canonical_hash).collect::<Outcome<Vec<_>>>()?;
    let report_value =
        molten::coordination::coordination_apply_report_value(molten::coordination::ApplyReportValueInput {
            decision,
            manifest_ref: &manifest_ref,
            final_state_ref: &final_state_ref,
            receipt_refs: &receipt_refs,
            assertion_refs: &assertion_refs,
            evidence_refs: &evidence_refs,
        })?;
    std::fs::create_dir_all(&out).map_err(molten::error::MoltenError::from)?;
    super::io::write_file(&out.join("report.preserves"), &molten::preserves_rail::to_text(&report_value)?)?;
    super::io::write_indexed_values(&out, "evidence", &evidence_values)?;
    println!(
        "coordination apply decision={} manifest={} state={} receipts={} assertions={} evidence={} out={}",
        decision,
        manifest_ref,
        final_state_ref,
        receipt_refs.len(),
        assertion_refs.len(),
        evidence_refs.len(),
        out.display()
    );
    Ok(())
}

fn wrong_handler(name: &str) -> molten::error::MoltenError {
    molten::error::MoltenError::invalid_harness(format!("coordination {name} handler called with another command"))
}
