pub(super) fn run(command: super::command::Top) -> molten::error::Result<()> {
    match command {
        super::command::Top::Record { out } => record(out),
        super::command::Top::Verify { fixture, receipt_out } => verify(fixture, receipt_out),
        super::command::Top::Tamper { fixture, kind, out } => tamper(fixture, &kind, out),
        super::command::Top::Rollup { receipts, out } => rollup(receipts, out),
        super::command::Top::Index { receipts, rollups, out } => index(receipts, rollups, out),
        super::command::Top::Compare {
            expected,
            actual,
            receipt_out,
        } => compare(expected, actual, receipt_out),
        super::command::Top::Explain { receipt, out } => explain(receipt, out),
        super::command::Top::Show { report } => show(report),
    }
}

fn record(out: std::path::PathBuf) -> molten::error::Result<()> {
    let fixture = molten::deterministic_replay::record_fixture_value()?;
    super::io::write_file(&out, &molten::preserves_rail::to_text(&fixture.value)?)?;
    println!(
        "deterministic replay fixture written to {} ref={} identity={} final_state={}",
        out.display(),
        fixture.record_ref,
        fixture.identity_ref,
        fixture.final_state_ref
    );
    Ok(())
}

fn verify(fixture: std::path::PathBuf, receipt_out: Option<std::path::PathBuf>) -> molten::error::Result<()> {
    let fixture_value = super::io::read_preserves_file(&fixture)?;
    let receipt = molten::deterministic_replay::verify_fixture_record_value(&fixture_value)?;
    let is_written_to_file = super::io::write_optional_preserves(receipt_out.as_ref(), &receipt.value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!(
            "deterministic replay verify ref={} decision={} divergence={}",
            receipt.receipt_ref,
            receipt.decision,
            receipt.divergence.as_str()
        ),
    );
    Ok(())
}

fn tamper(fixture: std::path::PathBuf, kind: &str, out: std::path::PathBuf) -> molten::error::Result<()> {
    super::io::read_preserves_file(&fixture)?;
    let variant = replay_fixture_variant_from_kind(kind)?;
    let tampered = molten::deterministic_replay::tampered_fixture_record_value(variant)?;
    super::io::write_file(&out, &molten::preserves_rail::to_text(&tampered.value)?)?;
    println!(
        "deterministic replay tampered fixture written to {} ref={} kind={} final_state={}",
        out.display(),
        tampered.record_ref,
        kind,
        tampered.final_state_ref
    );
    Ok(())
}

fn rollup(receipts: Vec<std::path::PathBuf>, out: std::path::PathBuf) -> molten::error::Result<()> {
    let mut inputs = Vec::with_capacity(receipts.len());
    for receipt in receipts {
        let value = super::io::read_preserves_file(&receipt)?;
        inputs.push(molten::deterministic_replay::ReplayRollupInput {
            expected_ref: Some(molten::preserves_rail::canonical_hash(&value)?),
            value,
        });
    }
    let rollup = molten::deterministic_replay::rollup_replay_receipts(&inputs)?;
    super::io::write_file(&out, &molten::preserves_rail::to_text(&rollup.value)?)?;
    println!(
        "deterministic replay rollup written to {} ref={} decision={} total={} pass={} deny={}",
        out.display(),
        rollup.rollup_ref,
        rollup.decision,
        rollup.total_count,
        rollup.pass_count,
        rollup.deny_count
    );
    Ok(())
}

fn index(
    receipts: Vec<std::path::PathBuf>,
    rollups: Vec<std::path::PathBuf>,
    out: std::path::PathBuf,
) -> molten::error::Result<()> {
    let mut inputs = Vec::with_capacity(receipts.len() + rollups.len());
    for receipt in receipts {
        let value = super::io::read_preserves_file(&receipt)?;
        inputs.push(molten::deterministic_replay::ReplayIndexInput {
            expected_ref: Some(molten::preserves_rail::canonical_hash(&value)?),
            value,
        });
    }
    for rollup in rollups {
        let value = super::io::read_preserves_file(&rollup)?;
        inputs.push(molten::deterministic_replay::ReplayIndexInput {
            expected_ref: Some(molten::preserves_rail::canonical_hash(&value)?),
            value,
        });
    }
    let index = molten::deterministic_replay::index_replay_evidence(&inputs)?;
    super::io::write_file(&out, &molten::preserves_rail::to_text(&index.value)?)?;
    println!(
        "deterministic replay index written to {} ref={} decision={} total={} pass={} deny={} raw_receipts={} rollups={}",
        out.display(),
        index.index_ref,
        index.decision,
        index.total_count,
        index.pass_count,
        index.deny_count,
        index.raw_receipt_count,
        index.rollup_count
    );
    Ok(())
}

fn compare(
    expected: std::path::PathBuf,
    actual: std::path::PathBuf,
    receipt_out: Option<std::path::PathBuf>,
) -> molten::error::Result<()> {
    let expected_value = super::io::read_preserves_file(&expected)?;
    let actual_value = super::io::read_preserves_file(&actual)?;
    let receipt = molten::deterministic_replay::compare_replay_fixture_values(expected_value, actual_value)?;
    let is_written_to_file = super::io::write_optional_preserves(receipt_out.as_ref(), &receipt.value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!(
            "deterministic replay compare ref={} decision={} first_divergence={}",
            receipt.receipt_ref,
            receipt.decision,
            receipt.first_divergence_ref.as_deref().unwrap_or("none")
        ),
    );
    Ok(())
}

fn explain(receipt: std::path::PathBuf, out: Option<std::path::PathBuf>) -> molten::error::Result<()> {
    let receipt_value = super::io::read_preserves_file(&receipt)?;
    let explain = molten::deterministic_replay::explain_replay_comparison_value(receipt_value)?;
    let is_written_to_file = super::io::write_optional_preserves(out.as_ref(), &explain.value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!(
            "deterministic replay explain ref={} decision={} first_divergence={}",
            explain.receipt_ref,
            explain.decision,
            explain.first_divergence_ref.as_deref().unwrap_or("none")
        ),
    );
    Ok(())
}

fn show(report: std::path::PathBuf) -> molten::error::Result<()> {
    let value = super::io::read_preserves_file(&report)?;
    let reference = molten::preserves_rail::canonical_hash(&value)?;
    println!("deterministic replay artifact ref={reference}");
    println!("{}", molten::preserves_rail::to_text(&value)?);
    Ok(())
}

fn replay_fixture_variant_from_kind(
    kind: &str,
) -> molten::error::Result<molten::deterministic_replay::ReplayFixtureVariant> {
    match kind {
        "identity" => Ok(molten::deterministic_replay::ReplayFixtureVariant::ChangedIdentity),
        "scheduler" => Ok(molten::deterministic_replay::ReplayFixtureVariant::ChangedScheduler),
        "input" => Ok(molten::deterministic_replay::ReplayFixtureVariant::ChangedInput),
        "effect-request" => Ok(molten::deterministic_replay::ReplayFixtureVariant::ChangedEffectRequest),
        "effect-response" => Ok(molten::deterministic_replay::ReplayFixtureVariant::ChangedEffectResponse),
        "policy" | "policy-decision" => Ok(molten::deterministic_replay::ReplayFixtureVariant::ChangedPolicyDecision),
        "action" => Ok(molten::deterministic_replay::ReplayFixtureVariant::ChangedAction),
        "receipt" => Ok(molten::deterministic_replay::ReplayFixtureVariant::ChangedReceipt),
        "output" => Ok(molten::deterministic_replay::ReplayFixtureVariant::ChangedOutput),
        "state" | "state-hash" => Ok(molten::deterministic_replay::ReplayFixtureVariant::ChangedStateHash),
        "live-effect" | "missing-effect" => {
            Ok(molten::deterministic_replay::ReplayFixtureVariant::MissingRecordedEffect)
        }
        _ => Err(molten::error::MoltenError::invalid_harness(format!(
            "unsupported replay fixture tamper kind {kind}"
        ))),
    }
}
