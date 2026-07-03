type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;
type Value<T> = preserves::Value<T>;

type OrderedMap<K, V> = std::collections::BTreeMap<K, V>;
type OrderedSet<T> = std::collections::BTreeSet<T>;

const TRACEABILITY_MANIFEST_SCHEMA: &str = "molten.testing.requirement-traceability.manifest.v1";
const TRACEABILITY_GATE_SCHEMA: &str = "molten.testing.requirement-traceability.gate.v1";
const VERIFICATION_RUN_RECEIPT_SCHEMA: &str = "molten.testing.verification-run-receipt.v1";
const AGGREGATE_PROOF_MANIFEST_SCHEMA: &str = "molten.testing.aggregate-proof-manifest.v1";
const LAYERED_PROOF_MANIFEST_SCHEMA: &str = "molten.evidence.layered-proof-manifest.v1";
const DENY_PATH_MATRIX_SCHEMA: &str = "molten.evidence.proof-deny-path-matrix.v1";
const MAX_REQUIREMENTS: usize = 4096;
const MAX_COVERAGE_ITEMS: usize = 4096;
const MAX_SUMMARY_LINES: usize = 8192;
const MAX_RECEIPT_ARGS: usize = 128;
const MAX_RECEIPT_REFS: usize = 256;
const MAX_PROOF_OBLIGATIONS: usize = 512;
const MAX_PROOF_LAYERS: usize = 128;
const VERIFICATION_RUN_RECEIPT_ARITY: usize = 13;
const _: () = assert!(MAX_REQUIREMENTS > 0);
const _: () = assert!(MAX_COVERAGE_ITEMS >= MAX_REQUIREMENTS);
const _: () = assert!(MAX_SUMMARY_LINES >= MAX_REQUIREMENTS);
const _: () = assert!(MAX_RECEIPT_ARGS > 0);
const _: () = assert!(MAX_RECEIPT_REFS > 0);
const _: () = assert!(MAX_PROOF_OBLIGATIONS > 0);
const _: () = assert!(MAX_PROOF_LAYERS > 0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecSource {
    pub source: String,
    pub markdown: String,
    pub changed: bool,
    pub default_kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequirementInput {
    pub id: String,
    pub source: String,
    pub kind: String,
    pub changed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationEvidence {
    pub target: String,
    pub command: String,
    pub artifact_ref: String,
    pub artifact_refs: Vec<String>,
    pub target_exists: bool,
    pub artifact_present: bool,
    pub source: String,
    pub receipt_ref: Option<String>,
    pub expected_decision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageInput {
    pub requirement_id: String,
    pub positive: Vec<VerificationEvidence>,
    pub negative: Vec<VerificationEvidence>,
    pub exemption: Option<CoverageExemption>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageExemption {
    pub class: String,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceabilityInput {
    pub requirements: Vec<RequirementInput>,
    pub coverage: Vec<CoverageInput>,
    pub require_receipt_backed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceabilityEntry {
    pub requirement_id: String,
    pub source: String,
    pub kind: String,
    pub changed: bool,
    pub status: String,
    pub diagnostics: Vec<String>,
    pub positive: Vec<VerificationEvidence>,
    pub negative: Vec<VerificationEvidence>,
    pub exemption: Option<CoverageExemption>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceabilityManifest {
    pub decision: String,
    pub entries: Vec<TraceabilityEntry>,
    pub summary: TraceabilitySummary,
    pub manifest_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TraceabilitySummary {
    pub covered: Vec<String>,
    pub exempt: Vec<String>,
    pub missing_positive: Vec<String>,
    pub missing_negative: Vec<String>,
    pub stale_reference: Vec<String>,
    pub unsupported: Vec<String>,
    pub compatibility_only: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationRunInput {
    pub requirement_id: String,
    pub coverage_kind: String,
    pub target: String,
    pub argv: Vec<String>,
    pub profile_ref: String,
    pub toolchain_refs: Vec<String>,
    pub exit_status: i64,
    pub stdout_ref: String,
    pub stderr_ref: String,
    pub artifact_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationRunReceipt {
    pub decision: String,
    pub requirement_id: String,
    pub coverage_kind: String,
    pub target: String,
    pub argv: Vec<String>,
    pub profile_ref: String,
    pub toolchain_refs: Vec<String>,
    pub exit_status: i64,
    pub stdout_ref: String,
    pub stderr_ref: String,
    pub artifact_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub receipt_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptCoverageSource {
    pub value: IoValue,
    pub target_exists: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofObligationInput {
    pub id: String,
    pub class: String,
    pub subject_ref: String,
    pub prerequisite_refs: Vec<String>,
    pub receipt_refs: Vec<String>,
    pub decision: String,
    pub requirement_ids: Vec<String>,
    pub coverage_kind: Option<String>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregateProofInput {
    pub manifest_id: String,
    pub subject_ref: String,
    pub required_obligation_ids: Vec<String>,
    pub obligations: Vec<ProofObligationInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregateProofManifest {
    pub decision: String,
    pub manifest_id: String,
    pub subject_ref: String,
    pub obligations: Vec<ProofObligationInput>,
    pub diagnostics: Vec<String>,
    pub manifest_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofLayerInput {
    pub id: String,
    pub role: String,
    pub subject_ref: String,
    pub decision: String,
    pub child_ids: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayeredProofInput {
    pub subject_ref: String,
    pub layers: Vec<ProofLayerInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayeredProofManifest {
    pub decision: String,
    pub subject_ref: String,
    pub layers: Vec<ProofLayerInput>,
    pub diagnostics: Vec<String>,
    pub manifest_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DenyPathCaseInput {
    pub class: String,
    pub fixture_ref: String,
    pub expected_decision: String,
    pub before_state_ref: Option<String>,
    pub after_state_ref: Option<String>,
    pub no_mutation_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DenyPathMatrixInput {
    pub gate: String,
    pub subject_ref: String,
    pub cases: Vec<DenyPathCaseInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DenyPathMatrix {
    pub decision: String,
    pub gate: String,
    pub subject_ref: String,
    pub diagnostics: Vec<String>,
    pub matrix_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofReadback {
    pub decision: String,
    pub entries: Vec<ProofReadbackEntry>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofReadbackEntry {
    pub requirement_id: String,
    pub status: String,
    pub positive_receipt_refs: Vec<String>,
    pub negative_receipt_refs: Vec<String>,
    pub artifact_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub caveats: Vec<String>,
}

pub fn requirements_from_sources(sources: &[SpecSource]) -> Result<Vec<RequirementInput>> {
    if sources.len() > MAX_REQUIREMENTS {
        return Err(MoltenError::invalid_harness(format!(
            "traceability source count {} exceeds bound {MAX_REQUIREMENTS}",
            sources.len()
        )));
    }
    let mut requirements = OrderedMap::new();
    for source in sources {
        validate_text("source", &source.source)?;
        validate_kind(&source.default_kind)?;
        for id in extract_requirement_ids(&source.markdown)? {
            let requirement = RequirementInput {
                id: id.clone(),
                source: source.source.clone(),
                kind: source.default_kind.clone(),
                changed: source.changed,
            };
            requirements.entry(id).or_insert(requirement);
        }
    }
    Ok(requirements.into_values().collect())
}

pub fn build_traceability_manifest(input: &TraceabilityInput) -> Result<TraceabilityManifest> {
    let requirement_map = requirement_map(&input.requirements)?;
    let mut coverage_map = coverage_map(&input.coverage)?;
    let mut entries = Vec::with_capacity(requirement_map.len());
    for requirement in requirement_map.values() {
        let coverage = coverage_map.remove(&requirement.id);
        entries.push(entry_for_requirement(requirement, coverage.as_ref(), input.require_receipt_backed)?);
    }
    for coverage in coverage_map.into_values() {
        entries.push(stale_coverage_entry(&coverage)?);
    }
    entries.sort_by(|left, right| left.requirement_id.cmp(&right.requirement_id));
    let summary = summarize_entries(&entries)?;
    let decision = traceability_decision(&summary).to_string();
    let value = manifest_value(&decision, &entries, &summary, input.require_receipt_backed)?;
    let manifest_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(TraceabilityManifest {
        decision,
        entries,
        summary,
        manifest_ref,
        value,
    })
}

pub fn traceability_gate_value(manifest: &TraceabilityManifest) -> Result<IoValue> {
    crate::preserves_rail::validate_content_ref(&manifest.manifest_ref)?;
    validate_decision(&manifest.decision)?;
    Ok(record("requirement-traceability-gate-v1", vec![
        string(TRACEABILITY_GATE_SCHEMA),
        record("decision", vec![string(&manifest.decision)]),
        record("manifest", vec![string(&manifest.manifest_ref)]),
        record("summary", vec![summary_value(&manifest.summary)?]),
        record("checks", vec![sequence(vec![
            check_value("positive-coverage-recorded", status(manifest.summary.missing_positive.is_empty())),
            check_value("negative-coverage-recorded", status(manifest.summary.missing_negative.is_empty())),
            check_value("stale-references-denied", status(manifest.summary.stale_reference.is_empty())),
            check_value("raw-coverage-claims-labeled", "pass"),
            check_value("documentation-exemptions-explicit", "pass"),
        ])]),
    ]))
}

pub fn render_summary(summary: &TraceabilitySummary) -> Result<String> {
    let mut lines = Vec::new();
    push_group(&mut lines, "covered", &summary.covered)?;
    push_group(&mut lines, "exempt", &summary.exempt)?;
    push_group(&mut lines, "missing-positive", &summary.missing_positive)?;
    push_group(&mut lines, "missing-negative", &summary.missing_negative)?;
    push_group(&mut lines, "stale-reference", &summary.stale_reference)?;
    push_group(&mut lines, "unsupported", &summary.unsupported)?;
    push_group(&mut lines, "compatibility-only", &summary.compatibility_only)?;
    Ok(lines.join("\n"))
}

pub fn compatibility_evidence(
    target: String,
    command: String,
    artifact_ref: String,
    target_exists: bool,
) -> VerificationEvidence {
    VerificationEvidence {
        artifact_refs: vec![artifact_ref.clone()],
        target,
        command,
        artifact_ref,
        target_exists,
        artifact_present: true,
        source: "compatibility".to_string(),
        receipt_ref: None,
        expected_decision: "compatibility".to_string(),
    }
}

pub fn build_verification_run_receipt(input: &VerificationRunInput) -> Result<VerificationRunReceipt> {
    validate_verification_run_input(input)?;
    let mut diagnostics = verification_run_diagnostics(input)?;
    diagnostics.sort();
    let decision = if diagnostics.is_empty() {
        expected_decision(&input.coverage_kind)?
    } else {
        "deny"
    }
    .to_string();
    let value = verification_run_receipt_value(input, &decision, &diagnostics)?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(VerificationRunReceipt {
        decision,
        requirement_id: input.requirement_id.clone(),
        coverage_kind: input.coverage_kind.clone(),
        target: input.target.clone(),
        argv: input.argv.clone(),
        profile_ref: input.profile_ref.clone(),
        toolchain_refs: input.toolchain_refs.clone(),
        exit_status: input.exit_status,
        stdout_ref: input.stdout_ref.clone(),
        stderr_ref: input.stderr_ref.clone(),
        artifact_refs: input.artifact_refs.clone(),
        diagnostics,
        receipt_ref,
        value,
    })
}

pub fn parse_verification_run_receipt(value: &IoValue) -> Result<VerificationRunReceipt> {
    let fields = value
        .collect_simple_record("verification-run-receipt-v1", Some(VERIFICATION_RUN_RECEIPT_ARITY))
        .ok_or_else(|| MoltenError::invalid_harness("expected <verification-run-receipt-v1 ...>"))?;
    require_schema(&fields[0], VERIFICATION_RUN_RECEIPT_SCHEMA, "verification run receipt")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    let requirement_id = record_string(&fields[2], "requirement")?;
    validate_requirement_id(&requirement_id)?;
    let coverage_kind = record_string(&fields[3], "coverage-kind")?;
    validate_coverage_kind(&coverage_kind)?;
    let target = record_string(&fields[4], "target")?;
    validate_text("verification target", &target)?;
    let argv = record_string_sequence(&fields[5], "argv")?;
    ensure_count_at_most(argv.len(), MAX_RECEIPT_ARGS, "verification argv")?;
    let profile_ref = record_ref(&fields[6], "profile")?;
    let toolchain_refs = record_ref_sequence(&fields[7], "toolchains")?;
    let exit_status = record_i64(&fields[8], "exit-status")?;
    let stdout_ref = record_ref(&fields[9], "stdout")?;
    let stderr_ref = record_ref(&fields[10], "stderr")?;
    let artifact_refs = record_ref_sequence(&fields[11], "artifacts")?;
    let diagnostics = record_string_sequence(&fields[12], "diagnostics")?;
    validate_verification_receipt_decision(&decision, &coverage_kind, exit_status, &diagnostics)?;
    Ok(VerificationRunReceipt {
        decision,
        requirement_id,
        coverage_kind,
        target,
        argv,
        profile_ref,
        toolchain_refs,
        exit_status,
        stdout_ref,
        stderr_ref,
        artifact_refs,
        diagnostics,
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        value: value.clone(),
    })
}

pub fn coverage_from_verification_receipts(sources: &[ReceiptCoverageSource]) -> Result<Vec<CoverageInput>> {
    let mut coverage = OrderedMap::<String, CoverageInput>::new();
    for source in sources {
        let receipt = parse_verification_run_receipt(&source.value)?;
        let evidence = evidence_from_verification_receipt(&receipt, source.target_exists)?;
        let entry = coverage.entry(receipt.requirement_id.clone()).or_insert_with(|| CoverageInput {
            requirement_id: receipt.requirement_id.clone(),
            positive: Vec::new(),
            negative: Vec::new(),
            exemption: None,
        });
        match receipt.coverage_kind.as_str() {
            "positive" => entry.positive.push(evidence),
            "negative" => entry.negative.push(evidence),
            other => return Err(MoltenError::invalid_harness(format!("unsupported coverage kind {other}"))),
        }
    }
    Ok(coverage.into_values().collect())
}

pub fn merge_coverage_inputs(inputs: Vec<CoverageInput>) -> Result<Vec<CoverageInput>> {
    let mut coverage = OrderedMap::<String, CoverageInput>::new();
    for input in inputs {
        validate_text("coverage requirement", &input.requirement_id)?;
        let entry = coverage.entry(input.requirement_id.clone()).or_insert_with(|| CoverageInput {
            requirement_id: input.requirement_id.clone(),
            positive: Vec::new(),
            negative: Vec::new(),
            exemption: None,
        });
        entry.positive.extend(input.positive);
        entry.negative.extend(input.negative);
        if input.exemption.is_some() {
            entry.exemption = input.exemption;
        }
    }
    Ok(coverage.into_values().collect())
}

pub fn build_aggregate_proof_manifest(input: &AggregateProofInput) -> Result<AggregateProofManifest> {
    validate_text("aggregate proof manifest id", &input.manifest_id)?;
    validate_ref(&input.subject_ref, "aggregate proof subject")?;
    ensure_count_at_most(input.obligations.len(), MAX_PROOF_OBLIGATIONS, "proof obligations")?;
    let mut diagnostics = aggregate_proof_diagnostics(input)?;
    diagnostics.sort();
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" }.to_string();
    let mut obligations = input.obligations.clone();
    obligations.sort_by(|left, right| left.id.cmp(&right.id));
    let value = aggregate_proof_value(input, &obligations, &decision, &diagnostics)?;
    let manifest_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(AggregateProofManifest {
        decision,
        manifest_id: input.manifest_id.clone(),
        subject_ref: input.subject_ref.clone(),
        obligations,
        diagnostics,
        manifest_ref,
        value,
    })
}

pub fn coverage_from_aggregate_proof(
    manifest: &AggregateProofManifest,
    target_exists: bool,
) -> Result<Vec<CoverageInput>> {
    let mut coverage = OrderedMap::<String, CoverageInput>::new();
    for obligation in &manifest.obligations {
        let Some(kind) = obligation.coverage_kind.as_deref() else {
            continue;
        };
        validate_coverage_kind(kind)?;
        for requirement_id in &obligation.requirement_ids {
            let entry = coverage.entry(requirement_id.clone()).or_insert_with(|| CoverageInput {
                requirement_id: requirement_id.clone(),
                positive: Vec::new(),
                negative: Vec::new(),
                exemption: None,
            });
            let evidence = VerificationEvidence {
                target: format!("aggregate-proof:{}", manifest.manifest_id),
                command: "molten test traceability scan --receipt aggregate-proof".to_string(),
                artifact_ref: manifest.manifest_ref.clone(),
                artifact_refs: obligation.receipt_refs.clone(),
                target_exists,
                artifact_present: crate::preserves_rail::validate_content_ref(&manifest.manifest_ref).is_ok(),
                source: "aggregate-proof".to_string(),
                receipt_ref: Some(manifest.manifest_ref.clone()),
                expected_decision: obligation_expected_decision(&obligation.class)?.to_string(),
            };
            match kind {
                "positive" => entry.positive.push(evidence),
                "negative" => entry.negative.push(evidence),
                other => return Err(MoltenError::invalid_harness(format!("unsupported aggregate proof kind {other}"))),
            }
        }
    }
    Ok(coverage.into_values().collect())
}

pub fn build_layered_proof_manifest(input: &LayeredProofInput) -> Result<LayeredProofManifest> {
    validate_ref(&input.subject_ref, "layered proof subject")?;
    ensure_count_at_most(input.layers.len(), MAX_PROOF_LAYERS, "proof layers")?;
    let mut diagnostics = layered_proof_diagnostics(input)?;
    diagnostics.sort();
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" }.to_string();
    let mut layers = input.layers.clone();
    layers.sort_by(|left, right| left.id.cmp(&right.id));
    let value = layered_proof_value(&input.subject_ref, &layers, &decision, &diagnostics)?;
    let manifest_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(LayeredProofManifest {
        decision,
        subject_ref: input.subject_ref.clone(),
        layers,
        diagnostics,
        manifest_ref,
        value,
    })
}

pub fn build_deny_path_matrix(input: &DenyPathMatrixInput) -> Result<DenyPathMatrix> {
    validate_text("deny path gate", &input.gate)?;
    validate_ref(&input.subject_ref, "deny path subject")?;
    ensure_count_at_most(input.cases.len(), MAX_COVERAGE_ITEMS, "deny path cases")?;
    let mut diagnostics = deny_path_diagnostics(input)?;
    diagnostics.sort();
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" }.to_string();
    let value = deny_path_matrix_value(input, &decision, &diagnostics)?;
    let matrix_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(DenyPathMatrix {
        decision,
        gate: input.gate.clone(),
        subject_ref: input.subject_ref.clone(),
        diagnostics,
        matrix_ref,
        value,
    })
}

pub fn build_proof_readback(manifest: &TraceabilityManifest) -> Result<ProofReadback> {
    let mut entries = Vec::with_capacity(manifest.entries.len());
    for entry in &manifest.entries {
        let positive_receipt_refs = receipt_refs(&entry.positive);
        let negative_receipt_refs = receipt_refs(&entry.negative);
        let artifact_refs = artifact_refs_for_entry(entry)?;
        let mut caveats = vec![
            "readback is non-normative".to_string(),
            "canonical receipts control pass or deny".to_string(),
        ];
        if entry
            .positive
            .iter()
            .chain(entry.negative.iter())
            .any(|evidence| evidence.source == "compatibility")
        {
            caveats.push("compatibility-only coverage must not be treated as receipt-backed proof".to_string());
        }
        entries.push(ProofReadbackEntry {
            requirement_id: entry.requirement_id.clone(),
            status: entry.status.clone(),
            positive_receipt_refs,
            negative_receipt_refs,
            artifact_refs,
            diagnostics: entry.diagnostics.clone(),
            caveats,
        });
    }
    entries.sort_by(|left, right| left.requirement_id.cmp(&right.requirement_id));
    Ok(ProofReadback {
        decision: manifest.decision.clone(),
        entries,
        caveats: vec![
            "summary is a rendered view over canonical traceability and proof receipts".to_string(),
            "readbacks do not grant authority, policy, provenance, resource, transport, source-gate, retention, or destructive-operation trust".to_string(),
        ],
    })
}

pub fn render_proof_readback(readback: &ProofReadback) -> Result<String> {
    let mut lines = vec![format!("proof-readback decision={}", readback.decision)];
    for caveat in &readback.caveats {
        validate_text("readback caveat", caveat)?;
        lines.push(format!("caveat: {caveat}"));
    }
    for entry in &readback.entries {
        validate_requirement_id(&entry.requirement_id)?;
        lines.push(format!("requirement {} status={}", entry.requirement_id, entry.status));
        lines.push(format!("  positive-receipts: {}", display_group(&entry.positive_receipt_refs)));
        lines.push(format!("  negative-receipts: {}", display_group(&entry.negative_receipt_refs)));
        lines.push(format!("  artifact-refs: {}", display_group(&entry.artifact_refs)));
        lines.push(format!("  diagnostics: {}", display_group(&entry.diagnostics)));
        lines.push(format!("  caveats: {}", display_group(&entry.caveats)));
    }
    Ok(lines.join("\n"))
}

fn requirement_map(requirements: &[RequirementInput]) -> Result<OrderedMap<String, RequirementInput>> {
    if requirements.is_empty() {
        return Err(MoltenError::invalid_harness("traceability manifest requires requirements"));
    }
    if requirements.len() > MAX_REQUIREMENTS {
        return Err(MoltenError::invalid_harness(format!(
            "traceability requirement count {} exceeds bound {MAX_REQUIREMENTS}",
            requirements.len()
        )));
    }
    let mut map = OrderedMap::new();
    for requirement in requirements {
        validate_requirement(requirement)?;
        if map.insert(requirement.id.clone(), requirement.clone()).is_some() {
            return Err(MoltenError::invalid_harness(format!("duplicate traceability requirement {}", requirement.id)));
        }
    }
    Ok(map)
}

fn coverage_map(coverage: &[CoverageInput]) -> Result<OrderedMap<String, CoverageInput>> {
    if coverage.len() > MAX_COVERAGE_ITEMS {
        return Err(MoltenError::invalid_harness(format!(
            "traceability coverage count {} exceeds bound {MAX_COVERAGE_ITEMS}",
            coverage.len()
        )));
    }
    let mut map = OrderedMap::new();
    for entry in coverage {
        validate_text("coverage requirement", &entry.requirement_id)?;
        if map.insert(entry.requirement_id.clone(), entry.clone()).is_some() {
            return Err(MoltenError::invalid_harness(format!(
                "duplicate traceability coverage entry {}",
                entry.requirement_id
            )));
        }
    }
    Ok(map)
}

fn entry_for_requirement(
    requirement: &RequirementInput,
    coverage: Option<&CoverageInput>,
    require_receipt_backed: bool,
) -> Result<TraceabilityEntry> {
    let mut diagnostics = Vec::new();
    let positive = coverage.map(|entry| entry.positive.clone()).unwrap_or_default();
    let negative = coverage.map(|entry| entry.negative.clone()).unwrap_or_default();
    let exemption = coverage.and_then(|entry| entry.exemption.clone());

    validate_evidence_list("positive", &positive, require_receipt_backed, &mut diagnostics)?;
    validate_evidence_list("negative", &negative, require_receipt_backed, &mut diagnostics)?;
    if let Some(exemption) = exemption.as_ref() {
        validate_exemption(exemption, &mut diagnostics)?;
    }

    let status = if diagnostics.iter().any(|diagnostic| diagnostic.starts_with("stale-")) {
        "stale-reference"
    } else if exemption.is_some() {
        "exempt"
    } else if requires_coverage(requirement) && positive.is_empty() {
        diagnostics.push("missing-positive-coverage".to_string());
        "missing-positive"
    } else if requires_coverage(requirement) && negative.is_empty() {
        diagnostics.push("missing-negative-coverage".to_string());
        "missing-negative"
    } else if requires_coverage(requirement) {
        "covered"
    } else if !positive.is_empty() || !negative.is_empty() {
        "covered"
    } else {
        "unsupported"
    };

    Ok(TraceabilityEntry {
        requirement_id: requirement.id.clone(),
        source: requirement.source.clone(),
        kind: requirement.kind.clone(),
        changed: requirement.changed,
        status: status.to_string(),
        diagnostics,
        positive,
        negative,
        exemption,
    })
}

fn stale_coverage_entry(coverage: &CoverageInput) -> Result<TraceabilityEntry> {
    let mut diagnostics = vec!["stale-requirement-id".to_string()];
    validate_evidence_list("positive", &coverage.positive, false, &mut diagnostics)?;
    validate_evidence_list("negative", &coverage.negative, false, &mut diagnostics)?;
    if let Some(exemption) = coverage.exemption.as_ref() {
        validate_exemption(exemption, &mut diagnostics)?;
    }
    Ok(TraceabilityEntry {
        requirement_id: coverage.requirement_id.clone(),
        source: "<stale-coverage>".to_string(),
        kind: "evidence".to_string(),
        changed: false,
        status: "stale-reference".to_string(),
        diagnostics,
        positive: coverage.positive.clone(),
        negative: coverage.negative.clone(),
        exemption: coverage.exemption.clone(),
    })
}

fn requires_coverage(requirement: &RequirementInput) -> bool {
    requirement.changed || requirement.kind == "evidence"
}

fn validate_evidence_list(
    label: &str,
    evidence: &[VerificationEvidence],
    require_receipt_backed: bool,
    diagnostics: &mut Vec<String>,
) -> Result<()> {
    if evidence.len() > MAX_COVERAGE_ITEMS {
        return Err(MoltenError::invalid_harness(format!(
            "traceability {label} evidence count {} exceeds bound {MAX_COVERAGE_ITEMS}",
            evidence.len()
        )));
    }
    let mut receipt_refs = OrderedSet::new();
    for item in evidence {
        validate_text("evidence target", &item.target)?;
        validate_text("evidence source", &item.source)?;
        if !item.target_exists {
            diagnostics.push(format!("stale-{label}-target:{}", item.target));
        }
        if item.command.trim().is_empty() {
            diagnostics.push(format!("stale-{label}-command:{}", item.target));
        }
        if item.source == "compatibility" && require_receipt_backed {
            diagnostics.push(format!("stale-{label}-compatibility-only:{}", item.target));
        }
        if !item.artifact_present {
            diagnostics.push(format!("stale-{label}-artifact:{}", item.target));
        }
        if item.artifact_ref.trim().is_empty() {
            diagnostics.push(format!("stale-{label}-artifact-ref:{}", item.target));
        } else if let Err(error) = crate::preserves_rail::validate_content_ref(&item.artifact_ref) {
            diagnostics.push(format!("stale-{label}-artifact-ref:{}:{error}", item.target));
        }
        for artifact_ref in &item.artifact_refs {
            if let Err(error) = crate::preserves_rail::validate_content_ref(artifact_ref) {
                diagnostics.push(format!("stale-{label}-artifact-ref:{}:{error}", item.target));
            }
        }
        if let Some(receipt_ref) = &item.receipt_ref {
            if !receipt_refs.insert(receipt_ref.clone()) {
                diagnostics.push(format!("stale-{label}-duplicate-receipt:{receipt_ref}"));
            }
            if let Err(error) = crate::preserves_rail::validate_content_ref(receipt_ref) {
                diagnostics.push(format!("stale-{label}-receipt-ref:{}:{error}", item.target));
            }
        } else if item.source != "compatibility" {
            diagnostics.push(format!("stale-{label}-missing-receipt-ref:{}", item.target));
        }
        let expected = expected_decision(label)?;
        if item.source != "compatibility" && item.expected_decision != expected {
            diagnostics.push(format!("stale-{label}-expected-decision:{}:{}", item.target, item.expected_decision));
        }
    }
    Ok(())
}

fn validate_exemption(exemption: &CoverageExemption, diagnostics: &mut Vec<String>) -> Result<()> {
    validate_text("exemption class", &exemption.class)?;
    validate_text("exemption evidence", &exemption.evidence)?;
    match exemption.class.as_str() {
        "documentation-only" | "operator-guidance" | "non-executable" => Ok(()),
        other => {
            diagnostics.push(format!("stale-exemption-class:{other}"));
            Ok(())
        }
    }
}

fn summarize_entries(entries: &[TraceabilityEntry]) -> Result<TraceabilitySummary> {
    if entries.len() > MAX_SUMMARY_LINES {
        return Err(MoltenError::invalid_harness(format!(
            "traceability summary entry count {} exceeds bound {MAX_SUMMARY_LINES}",
            entries.len()
        )));
    }
    let mut summary = TraceabilitySummary::default();
    for entry in entries {
        match entry.status.as_str() {
            "covered" => summary.covered.push(entry.requirement_id.clone()),
            "exempt" => summary.exempt.push(entry.requirement_id.clone()),
            "missing-positive" => summary.missing_positive.push(entry.requirement_id.clone()),
            "missing-negative" => summary.missing_negative.push(entry.requirement_id.clone()),
            "stale-reference" => summary.stale_reference.push(entry.requirement_id.clone()),
            "unsupported" => summary.unsupported.push(entry.requirement_id.clone()),
            other => return Err(MoltenError::invalid_harness(format!("unsupported traceability status {other}"))),
        }
        if entry
            .positive
            .iter()
            .chain(entry.negative.iter())
            .any(|evidence| evidence.source == "compatibility")
        {
            summary.compatibility_only.push(entry.requirement_id.clone());
        }
    }
    Ok(summary)
}

fn traceability_decision(summary: &TraceabilitySummary) -> &'static str {
    if summary.missing_positive.is_empty() && summary.missing_negative.is_empty() && summary.stale_reference.is_empty()
    {
        "pass"
    } else {
        "deny"
    }
}

fn manifest_value(
    decision: &str,
    entries: &[TraceabilityEntry],
    summary: &TraceabilitySummary,
    require_receipt_backed: bool,
) -> Result<IoValue> {
    validate_decision(decision)?;
    Ok(record("requirement-traceability-manifest-v1", vec![
        string(TRACEABILITY_MANIFEST_SCHEMA),
        record("decision", vec![string(decision)]),
        record("entries", vec![sequence(entry_values(entries)?)]),
        record("summary", vec![summary_value(summary)?]),
        record("policy", vec![record("receipt-backed-required", vec![
            crate::preserves_rail::bool_value(require_receipt_backed),
        ])]),
        record("checks", vec![sequence(vec![
            check_value("requirements-enumerated", status(!entries.is_empty())),
            check_value("positive-and-negative-required", "pass"),
            check_value("stale-references-fail-closed", "pass"),
            check_value("raw-coverage-claims-labeled", "pass"),
            check_value("receipt-backed-policy-explicit", "pass"),
        ])]),
    ]))
}

fn entry_values(entries: &[TraceabilityEntry]) -> Result<Vec<IoValue>> {
    let mut values = Vec::with_capacity(entries.len());
    for entry in entries {
        values.push(entry_value(entry)?);
    }
    Ok(values)
}

fn entry_value(entry: &TraceabilityEntry) -> Result<IoValue> {
    validate_text("entry requirement", &entry.requirement_id)?;
    validate_kind(&entry.kind)?;
    Ok(record("entry", vec![
        record("requirement", vec![string(&entry.requirement_id)]),
        record("source", vec![string(&entry.source)]),
        record("kind", vec![string(&entry.kind)]),
        record("changed", vec![crate::preserves_rail::bool_value(entry.changed)]),
        record("status", vec![string(&entry.status)]),
        record("positive", vec![sequence(evidence_values(&entry.positive)?)]),
        record("negative", vec![sequence(evidence_values(&entry.negative)?)]),
        record("exemption", vec![exemption_value(entry.exemption.as_ref())]),
        record("diagnostics", vec![sequence(entry.diagnostics.iter().map(string).collect())]),
    ]))
}

fn evidence_values(evidence: &[VerificationEvidence]) -> Result<Vec<IoValue>> {
    let mut values = Vec::with_capacity(evidence.len());
    for item in evidence {
        validate_text("evidence target", &item.target)?;
        values.push(record("evidence", vec![
            record("target", vec![string(&item.target)]),
            record("command", vec![string(&item.command)]),
            record("artifact", vec![string(&item.artifact_ref)]),
            record("artifacts", vec![sequence(string_values(&item.artifact_refs)?)]),
            record("target-exists", vec![crate::preserves_rail::bool_value(item.target_exists)]),
            record("artifact-present", vec![crate::preserves_rail::bool_value(item.artifact_present)]),
            record("source", vec![string(&item.source)]),
            record("receipt", vec![optional_ref_value(item.receipt_ref.as_deref())]),
            record("expected-decision", vec![string(&item.expected_decision)]),
        ]));
    }
    Ok(values)
}

fn exemption_value(exemption: Option<&CoverageExemption>) -> IoValue {
    match exemption {
        Some(value) => record("some", vec![
            record("class", vec![string(&value.class)]),
            record("evidence", vec![string(&value.evidence)]),
        ]),
        None => record("none", Vec::new()),
    }
}

fn summary_value(summary: &TraceabilitySummary) -> Result<IoValue> {
    Ok(record("summary", vec![
        group_value("covered", &summary.covered)?,
        group_value("exempt", &summary.exempt)?,
        group_value("missing-positive", &summary.missing_positive)?,
        group_value("missing-negative", &summary.missing_negative)?,
        group_value("stale-reference", &summary.stale_reference)?,
        group_value("unsupported", &summary.unsupported)?,
        group_value("compatibility-only", &summary.compatibility_only)?,
    ]))
}

fn group_value(label: &'static str, ids: &[String]) -> Result<IoValue> {
    Ok(record(label, vec![sequence(string_values(ids)?)]))
}

fn string_values(values: &[String]) -> Result<Vec<IoValue>> {
    let mut output = Vec::with_capacity(values.len());
    for value in values {
        validate_text("summary item", value)?;
        output.push(string(value));
    }
    Ok(output)
}

fn validate_verification_run_input(input: &VerificationRunInput) -> Result<()> {
    validate_requirement_id(&input.requirement_id)?;
    validate_coverage_kind(&input.coverage_kind)?;
    validate_text("verification target", &input.target)?;
    validate_string_list("verification argv", &input.argv, MAX_RECEIPT_ARGS)?;
    validate_ref(&input.profile_ref, "verification profile")?;
    validate_ref_list("verification toolchain refs", &input.toolchain_refs, MAX_RECEIPT_REFS)?;
    validate_ref(&input.stdout_ref, "verification stdout")?;
    validate_ref(&input.stderr_ref, "verification stderr")?;
    validate_ref_list("verification artifact refs", &input.artifact_refs, MAX_RECEIPT_REFS)?;
    Ok(())
}

fn verification_run_diagnostics(input: &VerificationRunInput) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    let is_success = input.exit_status == 0;
    match input.coverage_kind.as_str() {
        "positive" if !is_success => diagnostics.push("positive-run-exited-nonzero".to_string()),
        "negative" if is_success => diagnostics.push("negative-run-did-not-deny".to_string()),
        "positive" | "negative" => {}
        other => return Err(MoltenError::invalid_harness(format!("unsupported coverage kind {other}"))),
    }
    if input.artifact_refs.is_empty() {
        diagnostics.push("missing-produced-artifact-ref".to_string());
    }
    Ok(diagnostics)
}

fn verification_run_receipt_value(
    input: &VerificationRunInput,
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("verification-run-receipt-v1", vec![
        string(VERIFICATION_RUN_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("requirement", vec![string(&input.requirement_id)]),
        record("coverage-kind", vec![string(&input.coverage_kind)]),
        record("target", vec![string(&input.target)]),
        record("argv", vec![sequence(string_values(&input.argv)?)]),
        record("profile", vec![string(&input.profile_ref)]),
        record("toolchains", vec![sequence(string_values(&input.toolchain_refs)?)]),
        record("exit-status", vec![IoValue::new(input.exit_status)]),
        record("stdout", vec![string(&input.stdout_ref)]),
        record("stderr", vec![string(&input.stderr_ref)]),
        record("artifacts", vec![sequence(string_values(&input.artifact_refs)?)]),
        record("diagnostics", vec![sequence(string_values(diagnostics)?)]),
    ]))
}

fn validate_verification_receipt_decision(
    decision: &str,
    coverage_kind: &str,
    exit_status: i64,
    diagnostics: &[String],
) -> Result<()> {
    let expected = if diagnostics.is_empty() {
        match coverage_kind {
            "positive" if exit_status == 0 => "pass",
            "negative" if exit_status != 0 => "deny",
            "positive" | "negative" => "deny",
            other => return Err(MoltenError::invalid_harness(format!("unsupported coverage kind {other}"))),
        }
    } else {
        "deny"
    };
    if decision == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "verification receipt decision {decision} does not match expected {expected}"
        )))
    }
}

fn evidence_from_verification_receipt(
    receipt: &VerificationRunReceipt,
    target_exists: bool,
) -> Result<VerificationEvidence> {
    let artifact_ref = receipt.artifact_refs.first().cloned().unwrap_or_else(|| receipt.receipt_ref.clone());
    Ok(VerificationEvidence {
        target: receipt.target.clone(),
        command: receipt.argv.join(" "),
        artifact_ref,
        artifact_refs: receipt.artifact_refs.clone(),
        target_exists,
        artifact_present: receipt
            .artifact_refs
            .iter()
            .all(|reference| crate::preserves_rail::validate_content_ref(reference).is_ok()),
        source: "verification-run-receipt".to_string(),
        receipt_ref: Some(receipt.receipt_ref.clone()),
        expected_decision: receipt.decision.clone(),
    })
}

fn aggregate_proof_diagnostics(input: &AggregateProofInput) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    if input.obligations.is_empty() {
        diagnostics.push("missing-obligations".to_string());
    }
    let mut ids = OrderedSet::new();
    let mut obligation_map = OrderedMap::new();
    for obligation in &input.obligations {
        validate_obligation(obligation)?;
        if !ids.insert(obligation.id.clone()) {
            diagnostics.push(format!("duplicate-obligation:{}", obligation.id));
        }
        if obligation.subject_ref != input.subject_ref {
            diagnostics.push(format!("wrong-subject:{}", obligation.id));
        }
        if obligation.decision != obligation_expected_decision(&obligation.class)? {
            diagnostics.push(format!("wrong-expected-decision:{}", obligation.id));
        }
        obligation_map.insert(obligation.id.clone(), obligation);
    }
    for required in &input.required_obligation_ids {
        validate_text("required obligation id", required)?;
        if !obligation_map.contains_key(required) {
            diagnostics.push(format!("missing-child:{required}"));
        }
    }
    Ok(diagnostics)
}

fn validate_obligation(obligation: &ProofObligationInput) -> Result<()> {
    validate_text("obligation id", &obligation.id)?;
    validate_obligation_class(&obligation.class)?;
    validate_ref(&obligation.subject_ref, "obligation subject")?;
    validate_ref_list("obligation prerequisite refs", &obligation.prerequisite_refs, MAX_RECEIPT_REFS)?;
    validate_ref_list("obligation receipt refs", &obligation.receipt_refs, MAX_RECEIPT_REFS)?;
    validate_decision(&obligation.decision)?;
    for requirement_id in &obligation.requirement_ids {
        validate_requirement_id(requirement_id)?;
    }
    if let Some(kind) = obligation.coverage_kind.as_ref() {
        validate_coverage_kind(kind)?;
    }
    validate_string_list("obligation caveats", &obligation.caveats, MAX_RECEIPT_REFS)
}

fn obligation_expected_decision(class: &str) -> Result<&'static str> {
    match class {
        "fail-closed-negative" => Ok("deny"),
        "input-validation" | "canonicalization" | "admission" | "mutation-boundary" | "replay-determinism" => {
            Ok("pass")
        }
        other => Err(MoltenError::invalid_harness(format!("unsupported proof obligation class {other}"))),
    }
}

fn aggregate_proof_value(
    input: &AggregateProofInput,
    obligations: &[ProofObligationInput],
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("aggregate-proof-manifest-v1", vec![
        string(AGGREGATE_PROOF_MANIFEST_SCHEMA),
        record("decision", vec![string(decision)]),
        record("manifest-id", vec![string(&input.manifest_id)]),
        record("subject", vec![string(&input.subject_ref)]),
        record("required", vec![sequence(string_values(&input.required_obligation_ids)?)]),
        record("obligations", vec![sequence(obligation_values(obligations)?)]),
        record("diagnostics", vec![sequence(string_values(diagnostics)?)]),
        record("caveats", vec![sequence(vec![
            string("aggregate proof manifests are evidence only"),
            string("subsystem gates still control authority and side effects"),
        ])]),
    ]))
}

fn obligation_values(obligations: &[ProofObligationInput]) -> Result<Vec<IoValue>> {
    let mut values = Vec::with_capacity(obligations.len());
    for obligation in obligations {
        values.push(record("obligation", vec![
            record("id", vec![string(&obligation.id)]),
            record("class", vec![string(&obligation.class)]),
            record("subject", vec![string(&obligation.subject_ref)]),
            record("prerequisites", vec![sequence(string_values(&obligation.prerequisite_refs)?)]),
            record("receipts", vec![sequence(string_values(&obligation.receipt_refs)?)]),
            record("decision", vec![string(&obligation.decision)]),
            record("requirements", vec![sequence(string_values(&obligation.requirement_ids)?)]),
            record("coverage-kind", vec![optional_string_value(obligation.coverage_kind.as_deref())]),
            record("caveats", vec![sequence(string_values(&obligation.caveats)?)]),
        ]));
    }
    Ok(values)
}

fn layered_proof_diagnostics(input: &LayeredProofInput) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    if input.layers.is_empty() {
        diagnostics.push("missing-layers".to_string());
    }
    let mut ids = OrderedSet::new();
    let mut by_id = OrderedMap::new();
    for layer in &input.layers {
        validate_layer(layer)?;
        if !ids.insert(layer.id.clone()) {
            diagnostics.push(format!("duplicate-layer:{}", layer.id));
        }
        if layer.subject_ref != input.subject_ref {
            diagnostics.push(format!("wrong-subject:{}", layer.id));
        }
        if layer.role == "operator-readback" && layer.decision == "pass" {
            diagnostics.push(format!("diagnostic-readback-used-as-pass:{}", layer.id));
        }
        by_id.insert(layer.id.clone(), layer);
    }
    for layer in &input.layers {
        for child in &layer.child_ids {
            let Some(child_layer) = by_id.get(child) else {
                diagnostics.push(format!("stale-child:{}:{child}", layer.id));
                continue;
            };
            if child_layer.subject_ref != layer.subject_ref {
                diagnostics.push(format!("wrong-child-subject:{}:{child}", layer.id));
            }
            if !role_can_bind(&layer.role, &child_layer.role) {
                diagnostics.push(format!("unsupported-layer-link:{}:{}", layer.role, child_layer.role));
            }
        }
    }
    for cycle in layer_cycles(&by_id)? {
        diagnostics.push(format!("cycle:{cycle}"));
    }
    Ok(diagnostics)
}

fn validate_layer(layer: &ProofLayerInput) -> Result<()> {
    validate_text("proof layer id", &layer.id)?;
    validate_layer_role(&layer.role)?;
    validate_ref(&layer.subject_ref, "proof layer subject")?;
    validate_decision(&layer.decision)?;
    validate_string_list("proof layer child ids", &layer.child_ids, MAX_PROOF_LAYERS)?;
    validate_ref_list("proof layer evidence refs", &layer.evidence_refs, MAX_RECEIPT_REFS)?;
    validate_string_list("proof layer caveats", &layer.caveats, MAX_RECEIPT_REFS)
}

fn layer_cycles(by_id: &OrderedMap<String, &ProofLayerInput>) -> Result<Vec<String>> {
    let mut cycles = Vec::new();
    for id in by_id.keys() {
        let mut seen = OrderedSet::new();
        let mut stack = vec![id.clone()];
        while let Some(current) = stack.pop() {
            if !seen.insert(current.clone()) {
                cycles.push(id.clone());
                break;
            }
            if let Some(layer) = by_id.get(&current) {
                for child in &layer.child_ids {
                    stack.push(child.clone());
                }
            }
        }
    }
    cycles.sort();
    cycles.dedup();
    Ok(cycles)
}

fn role_can_bind(parent: &str, child: &str) -> bool {
    match parent {
        "pure-core" => false,
        "gate" => child == "pure-core",
        "replay" => child == "pure-core" || child == "gate",
        "release" => child == "pure-core" || child == "gate" || child == "replay",
        "operator-readback" => true,
        _ => false,
    }
}

fn layered_proof_value(
    subject_ref: &str,
    layers: &[ProofLayerInput],
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("layered-proof-manifest-v1", vec![
        string(LAYERED_PROOF_MANIFEST_SCHEMA),
        record("decision", vec![string(decision)]),
        record("subject", vec![string(subject_ref)]),
        record("layers", vec![sequence(layer_values(layers)?)]),
        record("diagnostics", vec![sequence(string_values(diagnostics)?)]),
        record("caveats", vec![sequence(vec![
            string("layered proof evidence does not promote trust automatically"),
            string("operator readbacks are non-normative summaries"),
        ])]),
    ]))
}

fn layer_values(layers: &[ProofLayerInput]) -> Result<Vec<IoValue>> {
    let mut values = Vec::with_capacity(layers.len());
    for layer in layers {
        values.push(record("layer", vec![
            record("id", vec![string(&layer.id)]),
            record("role", vec![string(&layer.role)]),
            record("subject", vec![string(&layer.subject_ref)]),
            record("decision", vec![string(&layer.decision)]),
            record("children", vec![sequence(string_values(&layer.child_ids)?)]),
            record("evidence", vec![sequence(string_values(&layer.evidence_refs)?)]),
            record("caveats", vec![sequence(string_values(&layer.caveats)?)]),
        ]));
    }
    Ok(values)
}

fn deny_path_diagnostics(input: &DenyPathMatrixInput) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    let mut classes = OrderedSet::new();
    for case in &input.cases {
        validate_deny_path_case(case)?;
        if !classes.insert(case.class.clone()) {
            diagnostics.push(format!("duplicate-deny-class:{}", case.class));
        }
        if case.expected_decision != "deny" {
            diagnostics.push(format!("wrong-deny-decision:{}", case.class));
        }
        if case.class == "denied-mutation" && !has_no_mutation_evidence(case) {
            diagnostics.push("missing-no-mutation-evidence:denied-mutation".to_string());
        }
    }
    for required in required_deny_classes() {
        if !classes.contains(*required) {
            diagnostics.push(format!("missing-deny-class:{required}"));
        }
    }
    Ok(diagnostics)
}

fn validate_deny_path_case(case: &DenyPathCaseInput) -> Result<()> {
    validate_deny_class(&case.class)?;
    validate_ref(&case.fixture_ref, "deny fixture")?;
    validate_decision(&case.expected_decision)?;
    if let Some(reference) = &case.before_state_ref {
        validate_ref(reference, "deny before state")?;
    }
    if let Some(reference) = &case.after_state_ref {
        validate_ref(reference, "deny after state")?;
    }
    if let Some(reference) = &case.no_mutation_ref {
        validate_ref(reference, "deny no-mutation receipt")?;
    }
    Ok(())
}

fn has_no_mutation_evidence(case: &DenyPathCaseInput) -> bool {
    case.no_mutation_ref.is_some()
        || matches!((&case.before_state_ref, &case.after_state_ref), (Some(before), Some(after)) if before == after)
}

fn deny_path_matrix_value(input: &DenyPathMatrixInput, decision: &str, diagnostics: &[String]) -> Result<IoValue> {
    Ok(record("proof-deny-path-matrix-v1", vec![
        string(DENY_PATH_MATRIX_SCHEMA),
        record("decision", vec![string(decision)]),
        record("gate", vec![string(&input.gate)]),
        record("subject", vec![string(&input.subject_ref)]),
        record("cases", vec![sequence(deny_case_values(&input.cases)?)]),
        record("diagnostics", vec![sequence(string_values(diagnostics)?)]),
        record("caveats", vec![sequence(vec![
            string("deny-path evidence proves only the declared gate scope"),
            string("logs are diagnostic-only"),
        ])]),
    ]))
}

fn deny_case_values(cases: &[DenyPathCaseInput]) -> Result<Vec<IoValue>> {
    let mut values = Vec::with_capacity(cases.len());
    for case in cases {
        values.push(record("case", vec![
            record("class", vec![string(&case.class)]),
            record("fixture", vec![string(&case.fixture_ref)]),
            record("expected-decision", vec![string(&case.expected_decision)]),
            record("before-state", vec![optional_ref_value(case.before_state_ref.as_deref())]),
            record("after-state", vec![optional_ref_value(case.after_state_ref.as_deref())]),
            record("no-mutation", vec![optional_ref_value(case.no_mutation_ref.as_deref())]),
        ]));
    }
    Ok(values)
}

fn required_deny_classes() -> &'static [&'static str] {
    &[
        "missing-artifact",
        "stale-ref",
        "malformed-schema",
        "wrong-signer",
        "wrong-purpose",
        "tampered-bytes",
        "duplicate-receipt",
        "denied-mutation",
        "diagnostic-only-not-pass",
    ]
}

fn receipt_refs(evidence: &[VerificationEvidence]) -> Vec<String> {
    evidence.iter().filter_map(|item| item.receipt_ref.clone()).collect()
}

fn artifact_refs_for_entry(entry: &TraceabilityEntry) -> Result<Vec<String>> {
    let mut refs = OrderedSet::new();
    for evidence in entry.positive.iter().chain(entry.negative.iter()) {
        for artifact_ref in &evidence.artifact_refs {
            validate_ref(artifact_ref, "readback artifact")?;
            refs.insert(artifact_ref.clone());
        }
    }
    Ok(refs.into_iter().collect())
}

fn extract_requirement_ids(markdown: &str) -> Result<Vec<String>> {
    let mut ids = OrderedSet::new();
    let mut rest = markdown;
    while let Some(start) = rest.find("r[") {
        let after_marker = &rest[start + "r[".len()..];
        let Some(end) = after_marker.find(']') else {
            return Err(MoltenError::invalid_harness("unterminated requirement marker r[...]"));
        };
        let id = &after_marker[..end];
        validate_requirement_id(id)?;
        ids.insert(id.to_string());
        rest = &after_marker[end + "]".len()..];
    }
    Ok(ids.into_iter().collect())
}

fn validate_requirement(requirement: &RequirementInput) -> Result<()> {
    validate_requirement_id(&requirement.id)?;
    validate_text("requirement source", &requirement.source)?;
    validate_kind(&requirement.kind)
}

fn validate_requirement_id(id: &str) -> Result<()> {
    validate_text("requirement id", id)?;
    if id.chars().any(char::is_whitespace) {
        return Err(MoltenError::invalid_harness(format!(
            "traceability requirement id {id} must not contain whitespace"
        )));
    }
    Ok(())
}

fn validate_kind(kind: &str) -> Result<()> {
    match kind {
        "evidence" | "documentation" | "operator" | "other" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported traceability requirement kind {other}"))),
    }
}

fn validate_coverage_kind(kind: &str) -> Result<()> {
    match kind {
        "positive" | "negative" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("coverage kind {other} must be positive or negative"))),
    }
}

fn expected_decision(kind: &str) -> Result<&'static str> {
    match kind {
        "positive" => Ok("pass"),
        "negative" => Ok("deny"),
        other => Err(MoltenError::invalid_harness(format!("unsupported coverage kind {other}"))),
    }
}

fn validate_decision(decision: &str) -> Result<()> {
    match decision {
        "pass" | "deny" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!(
            "unsupported traceability decision {other}; expected pass or deny"
        ))),
    }
}

fn validate_text(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(MoltenError::invalid_harness(format!("traceability {label} must not be empty")))
    } else {
        Ok(())
    }
}

fn validate_ref(reference: &str, label: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(reference)
        .map_err(|error| MoltenError::invalid_harness(format!("invalid {label} ref {reference}: {error}")))
}

fn validate_ref_list(label: &str, values: &[String], maximum: usize) -> Result<()> {
    ensure_count_at_most(values.len(), maximum, label)?;
    for value in values {
        validate_ref(value, label)?;
    }
    Ok(())
}

fn validate_string_list(label: &str, values: &[String], maximum: usize) -> Result<()> {
    ensure_count_at_most(values.len(), maximum, label)?;
    for value in values {
        validate_text(label, value)?;
    }
    Ok(())
}

fn validate_obligation_class(class: &str) -> Result<()> {
    match class {
        "input-validation"
        | "canonicalization"
        | "admission"
        | "mutation-boundary"
        | "replay-determinism"
        | "fail-closed-negative" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported proof obligation class {other}"))),
    }
}

fn validate_layer_role(role: &str) -> Result<()> {
    match role {
        "pure-core" | "gate" | "replay" | "release" | "operator-readback" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported proof layer role {other}"))),
    }
}

fn validate_deny_class(class: &str) -> Result<()> {
    if required_deny_classes().contains(&class) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported deny-path class {class}")))
    }
}

fn ensure_count_at_most(count: usize, maximum: usize, label: &str) -> Result<()> {
    if count > maximum {
        Err(MoltenError::invalid_harness(format!("{label} count {count} exceeds maximum {maximum}")))
    } else {
        Ok(())
    }
}

fn push_group(lines: &mut Vec<String>, label: &str, ids: &[String]) -> Result<()> {
    if lines.len() >= MAX_SUMMARY_LINES {
        return Err(MoltenError::invalid_harness("traceability summary exceeded line bound"));
    }
    if ids.is_empty() {
        lines.push(format!("{label}: none"));
    } else {
        lines.push(format!("{label}: {}", ids.join(", ")));
    }
    Ok(())
}

fn display_group(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join(", ")
    }
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn optional_string_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn require_schema(value: &Value<IoValue>, expected: &str, label: &str) -> Result<()> {
    let actual = value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected schema string for {label}")))?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} schema {actual} did not match {expected}")))
    }
}

fn record_string(value: &Value<IoValue>, label: &str) -> Result<String> {
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> field")))?;
    required_string(&record[0], label)
}

fn record_ref(value: &Value<IoValue>, label: &str) -> Result<String> {
    let reference = record_string(value, label)?;
    validate_ref(&reference, label)?;
    Ok(reference)
}

fn record_string_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> field")))?;
    let values = record[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {label}")))?;
    values.iter().map(|value| required_string(value, label)).collect()
}

fn record_ref_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let values = record_string_sequence(value, label)?;
    for reference in &values {
        validate_ref(reference, label)?;
    }
    Ok(values)
}

fn record_i64(value: &Value<IoValue>, label: &str) -> Result<i64> {
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> field")))?;
    record[0]
        .as_i64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected signed integer for {label}")))?
        .map_err(|_| MoltenError::invalid_harness(format!("signed integer for {label} is out of i64 range")))
}

fn required_string(value: &Value<IoValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value)
}

fn check_value(name: &'static str, state: &'static str) -> IoValue {
    record("check", vec![string(name), string(state)])
}

fn status(is_passing: bool) -> &'static str {
    if is_passing { "pass" } else { "deny" }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REQUIREMENT_ID: &str = "molten.testing.trace.fixture";
    const NEGATIVE_ID: &str = "molten.testing.trace.negative";
    const PROPERTY_CASES: u64 = 16;
    const PROPERTY_SALT_MAX: u64 = 1_000_000;

    fn local_ref(label: &str) -> String {
        crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
    }

    fn requirement(id: &str, kind: &str, changed: bool) -> RequirementInput {
        RequirementInput {
            id: id.to_string(),
            source: format!("cairn/specs/testing-harness/spec.md#{id}"),
            kind: kind.to_string(),
            changed,
        }
    }

    fn evidence(label: &str) -> VerificationEvidence {
        compatibility_evidence(format!("tests/{label}.rs"), format!("cargo test {label}"), local_ref(label), true)
    }

    fn receipt_input(requirement_id: &str, coverage_kind: &str, label: &str, exit_status: i64) -> VerificationRunInput {
        VerificationRunInput {
            requirement_id: requirement_id.to_string(),
            coverage_kind: coverage_kind.to_string(),
            target: format!("tests/{label}.rs"),
            argv: vec!["cargo".to_string(), "test".to_string(), label.to_string()],
            profile_ref: local_ref("profile"),
            toolchain_refs: vec![local_ref("toolchain")],
            exit_status,
            stdout_ref: local_ref(&format!("{label}-stdout")),
            stderr_ref: local_ref(&format!("{label}-stderr")),
            artifact_refs: vec![local_ref(&format!("{label}-artifact"))],
        }
    }

    fn proof_obligation(id: &str, class: &str, subject_ref: &str, decision: &str) -> ProofObligationInput {
        ProofObligationInput {
            id: id.to_string(),
            class: class.to_string(),
            subject_ref: subject_ref.to_string(),
            prerequisite_refs: vec![local_ref(&format!("{id}-pre"))],
            receipt_refs: vec![local_ref(&format!("{id}-receipt"))],
            decision: decision.to_string(),
            requirement_ids: vec![REQUIREMENT_ID.to_string()],
            coverage_kind: Some(if decision == "pass" { "positive" } else { "negative" }.to_string()),
            caveats: vec!["evidence-only".to_string()],
        }
    }

    fn proof_layer(id: &str, role: &str, subject_ref: &str, child_ids: Vec<String>) -> ProofLayerInput {
        ProofLayerInput {
            id: id.to_string(),
            role: role.to_string(),
            subject_ref: subject_ref.to_string(),
            decision: if role == "operator-readback" { "deny" } else { "pass" }.to_string(),
            child_ids,
            evidence_refs: vec![local_ref(&format!("{id}-evidence"))],
            caveats: vec!["evidence-only".to_string()],
        }
    }

    fn deny_case(class: &str) -> DenyPathCaseInput {
        let state_ref = local_ref("same-state");
        DenyPathCaseInput {
            class: class.to_string(),
            fixture_ref: local_ref(&format!("{class}-fixture")),
            expected_decision: "deny".to_string(),
            before_state_ref: Some(state_ref.clone()),
            after_state_ref: Some(state_ref),
            no_mutation_ref: Some(local_ref(&format!("{class}-no-mutation"))),
        }
    }

    #[test]
    fn complete_positive_and_negative_coverage_passes() {
        let manifest = build_traceability_manifest(&TraceabilityInput {
            requirements: vec![requirement(REQUIREMENT_ID, "evidence", true)],
            coverage: vec![CoverageInput {
                requirement_id: REQUIREMENT_ID.to_string(),
                positive: vec![evidence("positive")],
                negative: vec![evidence("negative")],
                exemption: None,
            }],
            require_receipt_backed: false,
        })
        .expect("traceability manifest");
        assert_eq!(manifest.decision, "pass");
        assert_eq!(manifest.summary.covered, vec![REQUIREMENT_ID.to_string()]);
        assert_eq!(manifest.summary.compatibility_only, vec![REQUIREMENT_ID.to_string()]);
        let gate = traceability_gate_value(&manifest).expect("traceability gate");
        assert!(
            crate::preserves_rail::to_text(&gate)
                .expect("render gate")
                .contains("requirement-traceability-gate-v1")
        );
    }

    #[test]
    fn missing_negative_coverage_denies_changed_requirement() {
        let manifest = build_traceability_manifest(&TraceabilityInput {
            requirements: vec![requirement(NEGATIVE_ID, "evidence", true)],
            coverage: vec![CoverageInput {
                requirement_id: NEGATIVE_ID.to_string(),
                positive: vec![evidence("positive-only")],
                negative: Vec::new(),
                exemption: None,
            }],
            require_receipt_backed: false,
        })
        .expect("traceability manifest");
        assert_eq!(manifest.decision, "deny");
        assert_eq!(manifest.summary.missing_negative, vec![NEGATIVE_ID.to_string()]);
    }

    #[test]
    fn stale_requirement_id_is_reported() {
        let manifest = build_traceability_manifest(&TraceabilityInput {
            requirements: vec![requirement(REQUIREMENT_ID, "evidence", true)],
            coverage: vec![CoverageInput {
                requirement_id: "molten.testing.trace.deleted".to_string(),
                positive: vec![evidence("stale-positive")],
                negative: vec![evidence("stale-negative")],
                exemption: None,
            }],
            require_receipt_backed: false,
        })
        .expect("traceability manifest");
        assert_eq!(manifest.decision, "deny");
        assert!(manifest.summary.stale_reference.iter().any(|id| id == "molten.testing.trace.deleted"));
    }

    #[test]
    fn missing_artifact_ref_is_stale() {
        let mut bad = evidence("missing-artifact");
        bad.artifact_present = false;
        let manifest = build_traceability_manifest(&TraceabilityInput {
            requirements: vec![requirement(REQUIREMENT_ID, "evidence", true)],
            coverage: vec![CoverageInput {
                requirement_id: REQUIREMENT_ID.to_string(),
                positive: vec![evidence("positive")],
                negative: vec![bad],
                exemption: None,
            }],
            require_receipt_backed: false,
        })
        .expect("traceability manifest");
        assert_eq!(manifest.decision, "deny");
        assert_eq!(manifest.summary.stale_reference, vec![REQUIREMENT_ID.to_string()]);
    }

    #[test]
    fn documentation_requirement_can_be_exempted() {
        let manifest = build_traceability_manifest(&TraceabilityInput {
            requirements: vec![requirement(REQUIREMENT_ID, "documentation", true)],
            coverage: vec![CoverageInput {
                requirement_id: REQUIREMENT_ID.to_string(),
                positive: Vec::new(),
                negative: Vec::new(),
                exemption: Some(CoverageExemption {
                    class: "documentation-only".to_string(),
                    evidence: "README.md#Testing".to_string(),
                }),
            }],
            require_receipt_backed: false,
        })
        .expect("traceability manifest");
        assert_eq!(manifest.decision, "pass");
        assert_eq!(manifest.summary.exempt, vec![REQUIREMENT_ID.to_string()]);
    }

    #[test]
    fn requirement_ids_are_extracted_from_markdown_sources() {
        let requirements = requirements_from_sources(&[SpecSource {
            source: "cairn/specs/testing-harness/spec.md".to_string(),
            markdown: "r[molten.testing.trace.fixture] text\nr[molten.testing.trace.negative] text".to_string(),
            changed: false,
            default_kind: "evidence".to_string(),
        }])
        .expect("extract requirements");
        assert_eq!(requirements.len(), [REQUIREMENT_ID, NEGATIVE_ID].len());
        assert!(requirements.iter().any(|requirement| requirement.id == REQUIREMENT_ID));
        assert!(requirements.iter().any(|requirement| requirement.id == NEGATIVE_ID));
    }

    #[test]
    fn verification_run_receipts_derive_receipt_backed_coverage() {
        let positive = build_verification_run_receipt(&receipt_input(REQUIREMENT_ID, "positive", "positive", 0))
            .expect("positive receipt");
        let negative = build_verification_run_receipt(&receipt_input(REQUIREMENT_ID, "negative", "negative", 1))
            .expect("negative receipt");
        let coverage = coverage_from_verification_receipts(&[
            ReceiptCoverageSource {
                value: positive.value,
                target_exists: true,
            },
            ReceiptCoverageSource {
                value: negative.value,
                target_exists: true,
            },
        ])
        .expect("receipt coverage");
        let manifest = build_traceability_manifest(&TraceabilityInput {
            requirements: vec![requirement(REQUIREMENT_ID, "evidence", true)],
            coverage,
            require_receipt_backed: true,
        })
        .expect("receipt-backed manifest");
        assert_eq!(manifest.decision, "pass");
        assert!(manifest.summary.compatibility_only.is_empty());
    }

    #[test]
    fn receipt_backed_policy_denies_raw_compatibility_tuples() {
        let manifest = build_traceability_manifest(&TraceabilityInput {
            requirements: vec![requirement(REQUIREMENT_ID, "evidence", true)],
            coverage: vec![CoverageInput {
                requirement_id: REQUIREMENT_ID.to_string(),
                positive: vec![evidence("positive")],
                negative: vec![evidence("negative")],
                exemption: None,
            }],
            require_receipt_backed: true,
        })
        .expect("manifest");
        assert_eq!(manifest.decision, "deny");
        assert_eq!(manifest.summary.stale_reference, vec![REQUIREMENT_ID.to_string()]);
    }

    #[test]
    fn verification_receipt_denies_wrong_exit_for_negative_coverage() {
        let receipt = build_verification_run_receipt(&receipt_input(REQUIREMENT_ID, "negative", "bad-negative", 0))
            .expect("negative receipt");
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic == "negative-run-did-not-deny"));
    }

    #[test]
    fn aggregate_proof_requires_all_children_and_subjects() {
        let subject = local_ref("subject");
        let manifest = build_aggregate_proof_manifest(&AggregateProofInput {
            manifest_id: "proof:complete".to_string(),
            subject_ref: subject.clone(),
            required_obligation_ids: vec!["validation".to_string(), "negative".to_string()],
            obligations: vec![
                proof_obligation("validation", "input-validation", &subject, "pass"),
                proof_obligation("negative", "fail-closed-negative", &subject, "deny"),
            ],
        })
        .expect("aggregate proof");
        assert_eq!(manifest.decision, "pass");
        let stale = build_aggregate_proof_manifest(&AggregateProofInput {
            manifest_id: "proof:stale".to_string(),
            subject_ref: subject,
            required_obligation_ids: vec!["validation".to_string(), "negative".to_string(), "replay".to_string()],
            obligations: manifest.obligations,
        })
        .expect("stale aggregate proof");
        assert_eq!(stale.decision, "deny");
        assert!(stale.diagnostics.iter().any(|diagnostic| diagnostic == "missing-child:replay"));
    }

    #[test]
    fn layered_proof_denies_cycles_and_readback_pass_promotion() {
        let subject = local_ref("layered-subject");
        let pass = build_layered_proof_manifest(&LayeredProofInput {
            subject_ref: subject.clone(),
            layers: vec![
                proof_layer("core", "pure-core", &subject, Vec::new()),
                proof_layer("gate", "gate", &subject, vec!["core".to_string()]),
                proof_layer("replay", "replay", &subject, vec!["gate".to_string()]),
                proof_layer("release", "release", &subject, vec!["replay".to_string()]),
            ],
        })
        .expect("layered proof");
        assert_eq!(pass.decision, "pass");
        let mut readback = proof_layer("readback", "operator-readback", &subject, vec!["release".to_string()]);
        readback.decision = "pass".to_string();
        let deny = build_layered_proof_manifest(&LayeredProofInput {
            subject_ref: subject,
            layers: vec![readback],
        })
        .expect("diagnostic layer proof");
        assert_eq!(deny.decision, "deny");
        assert!(deny.diagnostics.iter().any(|diagnostic| diagnostic.contains("diagnostic-readback-used-as-pass")));
    }

    #[test]
    fn deny_path_matrix_requires_no_mutation_and_all_classes() {
        let mut cases = required_deny_classes().iter().map(|class| deny_case(class)).collect::<Vec<_>>();
        let matrix = build_deny_path_matrix(&DenyPathMatrixInput {
            gate: "traceability".to_string(),
            subject_ref: local_ref("deny-subject"),
            cases: cases.clone(),
        })
        .expect("deny matrix");
        assert_eq!(matrix.decision, "pass");
        cases.retain(|case| case.class != "diagnostic-only-not-pass");
        let missing = build_deny_path_matrix(&DenyPathMatrixInput {
            gate: "traceability".to_string(),
            subject_ref: local_ref("deny-subject"),
            cases,
        })
        .expect("missing class matrix");
        assert_eq!(missing.decision, "deny");
        assert!(
            missing
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "missing-deny-class:diagnostic-only-not-pass")
        );
    }

    #[test]
    fn proof_readback_groups_requirement_evidence_and_caveats() {
        let positive = build_verification_run_receipt(&receipt_input(REQUIREMENT_ID, "positive", "positive", 0))
            .expect("positive receipt");
        let negative = build_verification_run_receipt(&receipt_input(REQUIREMENT_ID, "negative", "negative", 1))
            .expect("negative receipt");
        let coverage = coverage_from_verification_receipts(&[
            ReceiptCoverageSource {
                value: positive.value,
                target_exists: true,
            },
            ReceiptCoverageSource {
                value: negative.value,
                target_exists: true,
            },
        ])
        .expect("coverage");
        let manifest = build_traceability_manifest(&TraceabilityInput {
            requirements: vec![requirement(REQUIREMENT_ID, "evidence", true)],
            coverage,
            require_receipt_backed: true,
        })
        .expect("manifest");
        let readback = build_proof_readback(&manifest).expect("readback");
        let rendered = render_proof_readback(&readback).expect("rendered readback");
        assert!(rendered.contains("readback is non-normative"));
        assert!(rendered.contains(REQUIREMENT_ID));
    }

    #[hegel::test(test_cases = PROPERTY_CASES)]
    fn hegel_traceability_decision_law_and_deny_monotonicity(tc: hegel::TestCase) {
        let salt = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(PROPERTY_SALT_MAX));
        let id = format!("molten.testing.trace.generated.{salt}");
        let positive = build_verification_run_receipt(&receipt_input(&id, "positive", "generated-positive", 0))
            .expect("positive receipt");
        let negative = build_verification_run_receipt(&receipt_input(&id, "negative", "generated-negative", 1))
            .expect("negative receipt");
        let coverage = coverage_from_verification_receipts(&[
            ReceiptCoverageSource {
                value: positive.value,
                target_exists: true,
            },
            ReceiptCoverageSource {
                value: negative.value,
                target_exists: true,
            },
        ])
        .expect("coverage");
        let pass = build_traceability_manifest(&TraceabilityInput {
            requirements: vec![requirement(&id, "evidence", true)],
            coverage: coverage.clone(),
            require_receipt_backed: true,
        })
        .expect("pass manifest");
        assert_eq!(pass.decision, "pass");

        let mut stale_coverage = coverage;
        stale_coverage.push(CoverageInput {
            requirement_id: format!("molten.testing.trace.generated.stale.{salt}"),
            positive: vec![evidence("stale-positive")],
            negative: vec![evidence("stale-negative")],
            exemption: None,
        });
        let denied = build_traceability_manifest(&TraceabilityInput {
            requirements: vec![requirement(&id, "evidence", true)],
            coverage: stale_coverage,
            require_receipt_backed: true,
        })
        .expect("deny manifest");
        assert_eq!(denied.decision, "deny");
        assert!(!denied.summary.stale_reference.is_empty());
    }

    #[hegel::test(test_cases = PROPERTY_CASES)]
    fn hegel_receipt_ref_stability_and_binding_drift(tc: hegel::TestCase) {
        let salt = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(PROPERTY_SALT_MAX));
        let id = format!("molten.testing.receipt.generated.{salt}");
        let input = receipt_input(&id, "positive", "stable", 0);
        let first = build_verification_run_receipt(&input).expect("first");
        let second = build_verification_run_receipt(&input).expect("second");
        assert_eq!(first.receipt_ref, second.receipt_ref);
        let mut drift = input;
        drift.argv.push(format!("--salt={salt}"));
        let drifted = build_verification_run_receipt(&drift).expect("drifted");
        assert_ne!(first.receipt_ref, drifted.receipt_ref);
    }

    #[hegel::test(test_cases = PROPERTY_CASES)]
    fn hegel_layer_ordering_and_wrong_scope_denial(tc: hegel::TestCase) {
        let salt = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(PROPERTY_SALT_MAX));
        let subject = local_ref(&format!("subject-{salt}"));
        let other_subject = local_ref(&format!("other-subject-{salt}"));
        let mut core = proof_layer("core", "pure-core", &subject, Vec::new());
        if tc.draw(hegel::generators::booleans()) {
            core.subject_ref = other_subject;
        }
        let manifest = build_layered_proof_manifest(&LayeredProofInput {
            subject_ref: subject.clone(),
            layers: vec![proof_layer("gate", "gate", &subject, vec!["core".to_string()]), core],
        })
        .expect("layered manifest");
        if manifest.layers.iter().any(|layer| layer.subject_ref != subject) {
            assert_eq!(manifest.decision, "deny");
        }
        let rerender = build_layered_proof_manifest(&LayeredProofInput {
            subject_ref: subject.clone(),
            layers: manifest.layers.clone(),
        })
        .expect("rerender layered manifest");
        assert_eq!(manifest.manifest_ref, rerender.manifest_ref);
    }
}
