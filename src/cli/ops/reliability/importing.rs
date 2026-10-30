pub(super) struct Paths {
    pub expected: std::path::PathBuf,
    pub evidence: std::path::PathBuf,
    pub checker: std::path::PathBuf,
    pub reference_checker: Option<std::path::PathBuf>,
    pub work_root: std::path::PathBuf,
    pub ledger: std::path::PathBuf,
    pub receipt: std::path::PathBuf,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedInput {
    profile: molten::live_reliability::profile::Register,
    plan: molten::live_reliability::generator::OperationPlan,
    identities: std::collections::BTreeMap<String, String>,
    endpoints: std::collections::BTreeSet<String>,
    max_events: usize,
    max_witness_bytes: usize,
    max_recovery_ms: u64,
    readiness: std::collections::BTreeMap<String, molten::live_reliability::evidenceproof::ExpectedReadiness>,
}

pub(super) fn run(paths: Paths) -> molten::error::Result<()> {
    let input: ExpectedInput = super::read(&paths.expected)?;
    let evidence: molten::live_reliability::evidence::Observed = super::read(&paths.evidence)?;
    let admitted = molten::live_reliability::profile::admit(&input.profile)
        .map_err(|issue| molten::error::Failure::invalid_harness(format!("profile-denied:{issue:?}")))?;
    molten::live_reliability::generator::validate_plan(&admitted, &input.plan)
        .map_err(|issue| molten::error::Failure::invalid_harness(format!("plan-denied:{issue:?}")))?;
    let checked = super::checked(&paths.checker, &evidence.history, &paths.work_root)?;
    let reference_checked = paths.reference_checker.as_ref().map(|checker|
        super::checked(checker, &evidence.history, &paths.work_root)).transpose()?;
    let expected = molten::live_reliability::evidence::Expected {
        admitted, plan: input.plan, identities: input.identities, endpoints: input.endpoints,
        max_events: input.max_events, max_witness_bytes: input.max_witness_bytes,
        max_recovery_ms: input.max_recovery_ms, checked, reference_checked, readiness: input.readiness,
    };
    let receipt = molten::live_reliability::evidence::import(&expected, &evidence)
        .map_err(|issue| molten::error::Failure::invalid_harness(format!("evidence-denied:{issue:?}")))?;
    let ledger = molten::ledger::open_capability_ledger_root(&paths.ledger)?;
    molten::live_reliability::evidence::persist(&ledger, &receipt)?;
    let text = molten::preserves_rail::to_text(&receipt.value)?;
    std::fs::write(&paths.receipt, text).map_err(molten::error::Failure::from)?;
    println!("live reliability status={} verdict={} receipt={} production-admitted=false",
        receipt.status, receipt.verdict, receipt.receipt_ref);
    Ok(())
}
