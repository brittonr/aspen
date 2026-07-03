type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;
type Record<T> = preserves::Record<T>;
type Value<T> = preserves::Value<T>;

type OrderedMap<K, V> = std::collections::BTreeMap<K, V>;
type OrderedSet<T> = std::collections::BTreeSet<T>;

const VM_EVIDENCE_VALIDATION_SCHEMA: &str = "molten.testing.nixos-vm.evidence-validation.v1";
const VM_EVIDENCE_MANIFEST_SCHEMA: &str = "molten.testing.nixos-vm.evidence-manifest.v1";
const TOPOLOGY_ARITY: usize = 7;
const TOPOLOGY_SCHEMA_INDEX: usize = 0;
const TOPOLOGY_NODES_INDEX: usize = 1;
const TOPOLOGY_PACKAGE_INDEX: usize = 2;
const TOPOLOGY_NETWORK_INDEX: usize = 3;
const TOPOLOGY_CAVEATS_INDEX: usize = 5;
const NODE_EVIDENCE_ARITY: usize = 11;
const NODE_SCHEMA_INDEX: usize = 0;
const NODE_NAME_INDEX: usize = 1;
const NODE_STATE_ROOT_INDEX: usize = 2;
const NODE_STARTUP_INDEX: usize = 4;
const NODE_HEALTH_INDEX: usize = 5;
const NODE_CONTROL_LOOP_INDEX: usize = 6;
const NODE_HEARTBEAT_INDEX: usize = 7;
const NODE_LOGS_INDEX: usize = 9;
const TEST_RUN_ARITY: usize = 12;
const TEST_RUN_SCHEMA_INDEX: usize = 0;
const TEST_RUN_DECISION_INDEX: usize = 1;
const TEST_RUN_TOPOLOGY_INDEX: usize = 2;
const TEST_RUN_NODE_EVIDENCE_INDEX: usize = 5;
const TEST_RUN_CHILDREN_INDEX: usize = 6;
const TEST_RUN_REPLAY_INDEX: usize = 7;
const TEST_RUN_LOGS_INDEX: usize = 9;
const TEST_RUN_CAVEATS_INDEX: usize = 10;
const SOAK_RUN_DECISION_INDEX: usize = 1;
const SOAK_RUN_TOPOLOGY_INDEX: usize = 3;
const SOAK_RUN_NODE_EVIDENCE_INDEX: usize = 5;
const SOAK_RUN_REPLAY_INDEX: usize = 15;
const SOAK_RUN_CAVEATS_INDEX: usize = 18;
const MAX_VM_VALIDATION_ITEMS: usize = 512;
const _: () = assert!(MAX_VM_VALIDATION_ITEMS > 0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NixosVmEvidenceValidationInput<'a> {
    pub topology_value: &'a IoValue,
    pub node_evidence_values: &'a [IoValue],
    pub test_run_value: &'a IoValue,
    pub prod_soak_values: &'a [IoValue],
    pub expected_nodes: &'a [String],
    pub expected_package_ref: Option<&'a str>,
    pub expected_child_refs: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NixosVmEvidenceValidation {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub topology_ref: String,
    pub node_evidence_refs: Vec<String>,
    pub test_run_ref: String,
    pub prod_soak_refs: Vec<String>,
    pub validation_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmEvidenceManifestEntry {
    pub path: String,
    pub kind: String,
    pub content_ref: String,
    pub diagnostic_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedTopology {
    nodes: Vec<String>,
    package_ref: String,
    network: String,
    caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedNodeEvidence {
    node: String,
    state_root: String,
    startup_ref: String,
    health_ref: String,
    control_loop_ref: String,
    heartbeat_ref: String,
    log_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedTestRun {
    decision: String,
    topology_ref: String,
    node_evidence_refs: Vec<String>,
    child_refs: Vec<String>,
    replay_status: String,
    log_refs: Vec<String>,
    caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedProdSoakRun {
    decision: String,
    topology_ref: String,
    node_evidence_refs: Vec<String>,
    replay_status: String,
    caveats: Vec<String>,
}

pub fn validate_nixos_vm_evidence(input: &NixosVmEvidenceValidationInput<'_>) -> Result<NixosVmEvidenceValidation> {
    let topology = parse_topology(input.topology_value)?;
    let topology_ref = crate::preserves_rail::canonical_hash(input.topology_value)?;
    let parsed_nodes = parse_node_evidence_values(input.node_evidence_values)?;
    let node_evidence_refs = canonical_refs(input.node_evidence_values)?;
    let test_run = parse_test_run(input.test_run_value)?;
    let test_run_ref = crate::preserves_rail::canonical_hash(input.test_run_value)?;
    let prod_soaks = parse_prod_soaks(input.prod_soak_values)?;
    let prod_soak_refs = canonical_refs(input.prod_soak_values)?;
    let diagnostics = validation_diagnostics(ValidationContext {
        topology: &topology,
        topology_ref: &topology_ref,
        nodes: &parsed_nodes,
        node_refs: &node_evidence_refs,
        test_run: &test_run,
        prod_soaks: &prod_soaks,
        expected_nodes: input.expected_nodes,
        expected_package_ref: input.expected_package_ref,
        expected_child_refs: input.expected_child_refs,
    })?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" }.to_string();
    let value = vm_evidence_validation_value(ValidationValueInput {
        decision: &decision,
        diagnostics: &diagnostics,
        topology_ref: &topology_ref,
        node_evidence_refs: &node_evidence_refs,
        test_run_ref: &test_run_ref,
        prod_soak_refs: &prod_soak_refs,
    })?;
    let validation_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(NixosVmEvidenceValidation {
        decision,
        diagnostics,
        topology_ref,
        node_evidence_refs,
        test_run_ref,
        prod_soak_refs,
        validation_ref,
        value,
    })
}

pub fn vm_evidence_manifest_value(entries: &[VmEvidenceManifestEntry], caveats: &[String]) -> Result<IoValue> {
    validate_manifest_entries(entries)?;
    validate_strings("manifest caveat", caveats)?;
    Ok(record("nixos-vm-evidence-manifest-v1", vec![
        string(VM_EVIDENCE_MANIFEST_SCHEMA),
        record("artifacts", vec![sequence(manifest_entry_values(entries)?)]),
        record("caveats", vec![sequence(caveats.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            check_value("canonical-evidence-preserved", status(has_authoritative_entries(entries))),
            check_value("diagnostic-logs-marked", status(has_diagnostic_entries(entries))),
            check_value("manifest-does-not-grant-authority", "pass"),
        ])]),
    ]))
}

struct ValidationContext<'a> {
    topology: &'a ParsedTopology,
    topology_ref: &'a str,
    nodes: &'a [ParsedNodeEvidence],
    node_refs: &'a [String],
    test_run: &'a ParsedTestRun,
    prod_soaks: &'a [ParsedProdSoakRun],
    expected_nodes: &'a [String],
    expected_package_ref: Option<&'a str>,
    expected_child_refs: &'a [String],
}

struct ValidationValueInput<'a> {
    decision: &'a str,
    diagnostics: &'a [String],
    topology_ref: &'a str,
    node_evidence_refs: &'a [String],
    test_run_ref: &'a str,
    prod_soak_refs: &'a [String],
}

fn validation_diagnostics(input: ValidationContext<'_>) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    push_if(&mut diagnostics, input.test_run.decision != "pass", "vm-test-run-not-pass")?;
    push_if(
        &mut diagnostics,
        input.test_run.topology_ref != input.topology_ref,
        "test-run-topology-ref-mismatch",
    )?;
    push_if(&mut diagnostics, input.test_run.child_refs.is_empty(), "test-run-missing-child-workflow-refs")?;
    push_if(&mut diagnostics, input.test_run.replay_status.trim().is_empty(), "test-run-missing-replay-status")?;
    push_if(&mut diagnostics, input.test_run.log_refs.is_empty(), "test-run-missing-diagnostic-log-refs")?;
    push_if(&mut diagnostics, input.test_run.caveats.is_empty(), "test-run-missing-evidence-only-caveats")?;
    validate_topology_expectations(input.topology, input.expected_nodes, input.expected_package_ref, &mut diagnostics)?;
    validate_node_evidence(input.topology, input.nodes, input.node_refs, input.test_run, &mut diagnostics)?;
    validate_child_expectations(input.expected_child_refs, &input.test_run.child_refs, &mut diagnostics)?;
    validate_prod_soak_runs(input.topology_ref, input.node_refs, input.prod_soaks, &mut diagnostics)?;
    Ok(diagnostics)
}

fn validate_topology_expectations(
    topology: &ParsedTopology,
    expected_nodes: &[String],
    expected_package_ref: Option<&str>,
    diagnostics: &mut Vec<String>,
) -> Result<()> {
    if !expected_nodes.is_empty() {
        let actual = topology.nodes.iter().map(String::as_str).collect::<OrderedSet<_>>();
        let expected = expected_nodes.iter().map(String::as_str).collect::<OrderedSet<_>>();
        push_if(diagnostics, actual != expected, "topology-node-set-mismatch")?;
    }
    if let Some(package_ref) = expected_package_ref {
        push_if(diagnostics, topology.package_ref != package_ref, "topology-package-ref-mismatch")?;
    }
    push_if(diagnostics, topology.network.trim().is_empty(), "topology-missing-network")?;
    push_if(diagnostics, topology.caveats.is_empty(), "topology-missing-caveats")?;
    Ok(())
}

fn validate_node_evidence(
    topology: &ParsedTopology,
    nodes: &[ParsedNodeEvidence],
    node_refs: &[String],
    test_run: &ParsedTestRun,
    diagnostics: &mut Vec<String>,
) -> Result<()> {
    let topology_nodes = topology.nodes.iter().map(String::as_str).collect::<OrderedSet<_>>();
    let mut evidence_nodes = OrderedMap::new();
    for node in nodes {
        push_if(diagnostics, !topology_nodes.contains(node.node.as_str()), "node-evidence-outside-topology")?;
        push_if(diagnostics, node.state_root.trim().is_empty(), "node-evidence-missing-state-root")?;
        push_if(diagnostics, node.log_refs.is_empty(), "node-evidence-missing-diagnostic-log-refs")?;
        push_if(diagnostics, node.startup_ref == node.health_ref, "node-evidence-reuses-startup-health-ref")?;
        evidence_nodes.insert(node.node.as_str(), node);
    }
    push_if(diagnostics, evidence_nodes.len() != topology.nodes.len(), "node-evidence-count-mismatch")?;
    let node_ref_set = node_refs.iter().map(String::as_str).collect::<OrderedSet<_>>();
    for node_ref in &test_run.node_evidence_refs {
        push_if(diagnostics, !node_ref_set.contains(node_ref.as_str()), "test-run-node-ref-not-provided")?;
    }
    push_if(
        diagnostics,
        test_run.node_evidence_refs.len() != node_refs.len(),
        "test-run-node-ref-count-mismatch",
    )?;
    Ok(())
}

fn validate_child_expectations(
    expected_child_refs: &[String],
    actual_child_refs: &[String],
    diagnostics: &mut Vec<String>,
) -> Result<()> {
    let actual = actual_child_refs.iter().map(String::as_str).collect::<OrderedSet<_>>();
    for child_ref in expected_child_refs {
        push_if(diagnostics, !actual.contains(child_ref.as_str()), "expected-child-ref-missing")?;
    }
    Ok(())
}

fn validate_prod_soak_runs(
    topology_ref: &str,
    node_refs: &[String],
    prod_soaks: &[ParsedProdSoakRun],
    diagnostics: &mut Vec<String>,
) -> Result<()> {
    let node_ref_set = node_refs.iter().map(String::as_str).collect::<OrderedSet<_>>();
    for run in prod_soaks {
        push_if(diagnostics, run.decision != "pass", "prod-soak-run-not-pass")?;
        push_if(diagnostics, run.topology_ref != topology_ref, "prod-soak-topology-ref-mismatch")?;
        push_if(diagnostics, run.node_evidence_refs.is_empty(), "prod-soak-missing-node-evidence")?;
        for node_ref in &run.node_evidence_refs {
            push_if(diagnostics, !node_ref_set.contains(node_ref.as_str()), "prod-soak-node-ref-not-provided")?;
        }
        push_if(diagnostics, run.replay_status.trim().is_empty(), "prod-soak-missing-replay-status")?;
        push_if(diagnostics, run.caveats.is_empty(), "prod-soak-missing-caveats")?;
    }
    Ok(())
}

fn parse_topology(value: &IoValue) -> Result<ParsedTopology> {
    let topology = simple_record(value, "nixos-vm-topology-v1", TOPOLOGY_ARITY)?;
    require_schema(&topology[TOPOLOGY_SCHEMA_INDEX], crate::preserves_rail::NIXOS_VM_TOPOLOGY_SCHEMA, "topology")?;
    let nodes = node_names(&topology[TOPOLOGY_NODES_INDEX])?;
    let package = simple_field_record(&topology[TOPOLOGY_PACKAGE_INDEX], "package", "topology package")?;
    let package_record = value_to_iovalue(&package[0]);
    let molten_package = simple_record(&package_record, "molten-package", 2)?;
    let package_ref = required_record_string(&molten_package[0], "ref", "topology package ref")?;
    let network = required_record_string(&topology[TOPOLOGY_NETWORK_INDEX], "network", "topology network")?;
    let caveats = required_string_sequence_record(&topology[TOPOLOGY_CAVEATS_INDEX], "caveats", "topology caveats")?;
    Ok(ParsedTopology {
        nodes,
        package_ref,
        network,
        caveats,
    })
}

fn parse_node_evidence_values(values: &[IoValue]) -> Result<Vec<ParsedNodeEvidence>> {
    if values.len() > MAX_VM_VALIDATION_ITEMS {
        return Err(MoltenError::invalid_harness(format!(
            "VM node evidence count {} exceeds bound {MAX_VM_VALIDATION_ITEMS}",
            values.len()
        )));
    }
    let mut output = Vec::with_capacity(values.len());
    for value in values {
        output.push(parse_node_evidence(value)?);
    }
    Ok(output)
}

fn parse_node_evidence(value: &IoValue) -> Result<ParsedNodeEvidence> {
    let node = simple_record(value, "nixos-vm-node-evidence-v1", NODE_EVIDENCE_ARITY)?;
    require_schema(&node[NODE_SCHEMA_INDEX], crate::preserves_rail::NIXOS_VM_NODE_EVIDENCE_SCHEMA, "node evidence")?;
    Ok(ParsedNodeEvidence {
        node: required_record_string(&node[NODE_NAME_INDEX], "node", "node evidence node")?,
        state_root: required_record_string(&node[NODE_STATE_ROOT_INDEX], "state-root", "node evidence state root")?,
        startup_ref: required_record_ref(&node[NODE_STARTUP_INDEX], "startup-receipt", "node startup")?,
        health_ref: required_record_ref(&node[NODE_HEALTH_INDEX], "health-receipt", "node health")?,
        control_loop_ref: required_record_ref(
            &node[NODE_CONTROL_LOOP_INDEX],
            "control-loop-receipt",
            "node control loop",
        )?,
        heartbeat_ref: required_record_ref(&node[NODE_HEARTBEAT_INDEX], "heartbeat-receipt", "node heartbeat")?,
        log_refs: required_ref_sequence_record(&node[NODE_LOGS_INDEX], "logs", "node logs")?,
    })
}

fn parse_test_run(value: &IoValue) -> Result<ParsedTestRun> {
    let run = simple_record(value, "nixos-vm-test-run-v1", TEST_RUN_ARITY)?;
    require_schema(&run[TEST_RUN_SCHEMA_INDEX], crate::preserves_rail::NIXOS_VM_TEST_RUN_SCHEMA, "test run")?;
    Ok(ParsedTestRun {
        decision: required_record_string(&run[TEST_RUN_DECISION_INDEX], "decision", "test run decision")?,
        topology_ref: required_record_ref(&run[TEST_RUN_TOPOLOGY_INDEX], "topology", "test run topology")?,
        node_evidence_refs: required_ref_sequence_record(
            &run[TEST_RUN_NODE_EVIDENCE_INDEX],
            "node-evidence",
            "test run nodes",
        )?,
        child_refs: required_ref_sequence_record(
            &run[TEST_RUN_CHILDREN_INDEX],
            "child-workflows",
            "test run children",
        )?,
        replay_status: required_record_string(&run[TEST_RUN_REPLAY_INDEX], "replay-status", "test run replay status")?,
        log_refs: required_ref_sequence_record(&run[TEST_RUN_LOGS_INDEX], "logs", "test run logs")?,
        caveats: required_string_sequence_record(&run[TEST_RUN_CAVEATS_INDEX], "caveats", "test run caveats")?,
    })
}

fn parse_prod_soaks(values: &[IoValue]) -> Result<Vec<ParsedProdSoakRun>> {
    if values.len() > MAX_VM_VALIDATION_ITEMS {
        return Err(MoltenError::invalid_harness(format!(
            "VM prod-soak count {} exceeds bound {MAX_VM_VALIDATION_ITEMS}",
            values.len()
        )));
    }
    let mut output = Vec::with_capacity(values.len());
    for value in values {
        output.push(parse_prod_soak_run(value)?);
    }
    Ok(output)
}

fn parse_prod_soak_run(value: &IoValue) -> Result<ParsedProdSoakRun> {
    let run = value
        .collect_simple_record("prod-soak-run-v1", None)
        .ok_or_else(|| MoltenError::invalid_harness("expected prod-soak-run-v1 receipt"))?;
    require_schema(&run[0], crate::preserves_rail::PROD_SOAK_RUN_SCHEMA, "prod soak run")?;
    Ok(ParsedProdSoakRun {
        decision: required_record_string(&run[SOAK_RUN_DECISION_INDEX], "decision", "prod soak decision")?,
        topology_ref: required_record_ref(&run[SOAK_RUN_TOPOLOGY_INDEX], "topology", "prod soak topology")?,
        node_evidence_refs: required_ref_sequence_record(
            &run[SOAK_RUN_NODE_EVIDENCE_INDEX],
            "node-evidence",
            "prod soak nodes",
        )?,
        replay_status: required_record_string(&run[SOAK_RUN_REPLAY_INDEX], "replay-status", "prod soak replay status")?,
        caveats: required_string_sequence_record(&run[SOAK_RUN_CAVEATS_INDEX], "caveats", "prod soak caveats")?,
    })
}

fn vm_evidence_validation_value(input: ValidationValueInput<'_>) -> Result<IoValue> {
    crate::preserves_rail::validate_content_ref(input.topology_ref)?;
    crate::preserves_rail::validate_content_ref(input.test_run_ref)?;
    validate_ref_list("node evidence", input.node_evidence_refs)?;
    validate_ref_list("prod soak", input.prod_soak_refs)?;
    validate_strings("validation diagnostic", input.diagnostics)?;
    Ok(record("nixos-vm-evidence-validation-v1", vec![
        string(VM_EVIDENCE_VALIDATION_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("topology", vec![string(input.topology_ref)]),
        record("node-evidence", vec![sequence(input.node_evidence_refs.iter().map(string).collect())]),
        record("test-run", vec![string(input.test_run_ref)]),
        record("prod-soak", vec![sequence(input.prod_soak_refs.iter().map(string).collect())]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            check_value("canonical-receipts-parsed", "pass"),
            check_value("topology-bound", status(input.decision == "pass")),
            check_value("logs-diagnostic-only", "pass"),
            check_value("evidence-does-not-grant-authority", "pass"),
        ])]),
    ]))
}

fn canonical_refs(values: &[IoValue]) -> Result<Vec<String>> {
    let mut refs = Vec::with_capacity(values.len());
    for value in values {
        refs.push(crate::preserves_rail::canonical_hash(value)?);
    }
    Ok(refs)
}

fn validate_manifest_entries(entries: &[VmEvidenceManifestEntry]) -> Result<()> {
    if entries.is_empty() {
        return Err(MoltenError::invalid_harness("VM evidence manifest requires entries"));
    }
    if entries.len() > MAX_VM_VALIDATION_ITEMS {
        return Err(MoltenError::invalid_harness(format!(
            "VM evidence manifest entry count {} exceeds bound {MAX_VM_VALIDATION_ITEMS}",
            entries.len()
        )));
    }
    let mut paths = OrderedSet::new();
    for entry in entries {
        validate_text("manifest path", &entry.path)?;
        validate_text("manifest kind", &entry.kind)?;
        crate::preserves_rail::validate_content_ref(&entry.content_ref)?;
        if !paths.insert(entry.path.as_str()) {
            return Err(MoltenError::invalid_harness(format!("duplicate VM evidence manifest path {}", entry.path)));
        }
    }
    Ok(())
}

fn manifest_entry_values(entries: &[VmEvidenceManifestEntry]) -> Result<Vec<IoValue>> {
    let mut values = Vec::with_capacity(entries.len());
    for entry in entries {
        values.push(record("artifact", vec![
            record("path", vec![string(&entry.path)]),
            record("kind", vec![string(&entry.kind)]),
            record("ref", vec![string(&entry.content_ref)]),
            record("diagnostic-only", vec![crate::preserves_rail::bool_value(entry.diagnostic_only)]),
        ]));
    }
    Ok(values)
}

fn has_authoritative_entries(entries: &[VmEvidenceManifestEntry]) -> bool {
    entries.iter().any(|entry| !entry.diagnostic_only)
}

fn has_diagnostic_entries(entries: &[VmEvidenceManifestEntry]) -> bool {
    entries.iter().any(|entry| entry.diagnostic_only)
}

fn node_names(value: &Value<IoValue>) -> Result<Vec<String>> {
    let sequence = required_sequence_record(value, "nodes", "topology nodes")?;
    let mut nodes = Vec::with_capacity(sequence.len());
    for item in sequence.iter() {
        let node = item
            .collect_simple_record("node", Some(1))
            .ok_or_else(|| MoltenError::invalid_harness("topology node must be <node string>"))?;
        nodes.push(required_string(&node[0], "topology node")?);
    }
    Ok(nodes)
}

fn required_ref_sequence_record(value: &Value<IoValue>, label: &str, context: &str) -> Result<Vec<String>> {
    let refs = required_string_sequence_record(value, label, context)?;
    validate_ref_list(context, &refs)?;
    Ok(refs)
}

fn required_string_sequence_record(value: &Value<IoValue>, label: &str, context: &str) -> Result<Vec<String>> {
    let sequence = required_sequence_record(value, label, context)?;
    let mut output = Vec::with_capacity(sequence.len());
    for item in sequence.iter() {
        output.push(required_string(item, context)?);
    }
    Ok(output)
}

fn required_sequence_record(value: &Value<IoValue>, label: &str, context: &str) -> Result<Vec<Value<IoValue>>> {
    let record = simple_field_record(value, label, context)?;
    let sequence = record[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {context}")))?;
    Ok(sequence.into_owned())
}

fn required_record_ref(value: &Value<IoValue>, label: &str, context: &str) -> Result<String> {
    let reference = required_record_string(value, label, context)?;
    crate::preserves_rail::validate_content_ref(&reference)?;
    Ok(reference)
}

fn required_record_string(value: &Value<IoValue>, label: &str, context: &str) -> Result<String> {
    let record = simple_field_record(value, label, context)?;
    required_string(&record[0], context)
}

fn require_schema(value: &Value<IoValue>, expected: &str, context: &str) -> Result<()> {
    let actual = required_string(value, context)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unexpected {context} schema {actual}; expected {expected}")))
    }
}

fn simple_record<'a>(
    value: &'a IoValue,
    label: &str,
    arity: usize,
) -> Result<std::borrow::Cow<'a, Record<Value<IoValue>>>> {
    value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> with arity {arity}")))
}

fn simple_field_record<'a>(
    value: &'a Value<IoValue>,
    label: &str,
    context: &str,
) -> Result<std::borrow::Cow<'a, Record<Value<IoValue>>>> {
    value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> for {context}")))
}

fn required_string(value: &Value<IoValue>, context: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {context}")))
}

fn validate_ref_list(label: &str, refs: &[String]) -> Result<()> {
    if refs.len() > MAX_VM_VALIDATION_ITEMS {
        return Err(MoltenError::invalid_harness(format!(
            "VM {label} ref count {} exceeds bound {MAX_VM_VALIDATION_ITEMS}",
            refs.len()
        )));
    }
    for reference in refs {
        crate::preserves_rail::validate_content_ref(reference)?;
    }
    Ok(())
}

fn validate_strings(label: &str, values: &[String]) -> Result<()> {
    if values.len() > MAX_VM_VALIDATION_ITEMS {
        return Err(MoltenError::invalid_harness(format!(
            "VM {label} count {} exceeds bound {MAX_VM_VALIDATION_ITEMS}",
            values.len()
        )));
    }
    for value in values {
        validate_text(label, value)?;
    }
    Ok(())
}

fn validate_text(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(MoltenError::invalid_harness(format!("VM {label} must not be empty")))
    } else {
        Ok(())
    }
}

fn push_if(diagnostics: &mut Vec<String>, condition: bool, diagnostic: &'static str) -> Result<()> {
    if condition {
        if diagnostics.len() >= MAX_VM_VALIDATION_ITEMS {
            return Err(MoltenError::invalid_harness("VM validation diagnostics exceeded bound"));
        }
        diagnostics.push(diagnostic.to_string());
    }
    Ok(())
}

fn value_to_iovalue(value: &Value<IoValue>) -> IoValue {
    crate::preserves_rail::value_to_iovalue(value)
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

    const NODE_A: &str = "node_a";
    const NODE_B: &str = "node_b";
    const NETWORK: &str = "nixos-test-private";
    const STATE_ROOT: &str = "/var/lib/molten";
    const SCENARIO: &str = "phase2-live-control-service-job-restart";

    fn local_ref(label: &str) -> String {
        crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
    }

    fn topology() -> IoValue {
        crate::nixos_vm::topology_value(&crate::nixos_vm::NixosVmTopologyInput {
            nodes: &[NODE_A.to_string(), NODE_B.to_string()],
            package_ref: &local_ref("package"),
            package_path: "/nix/store/example-molten",
            network: NETWORK,
            nix_inputs: &["source:locked".to_string()],
            caveats: &["vm evidence is platform integration evidence only".to_string()],
        })
        .expect("topology value")
    }

    fn node_evidence(node: &str, label: &str) -> IoValue {
        crate::nixos_vm::node_evidence_value(&crate::nixos_vm::NixosVmNodeEvidenceInput {
            node,
            state_root: STATE_ROOT,
            identity_receipt_ref: Some(&local_ref(&format!("{label}-identity"))),
            startup_receipt_ref: &local_ref(&format!("{label}-startup")),
            health_receipt_ref: &local_ref(&format!("{label}-health")),
            control_loop_receipt_ref: &local_ref(&format!("{label}-loop")),
            heartbeat_receipt_ref: &local_ref(&format!("{label}-heartbeat")),
            shutdown_receipt_ref: Some(&local_ref(&format!("{label}-shutdown"))),
            log_refs: &[local_ref(&format!("{label}-log"))],
        })
        .expect("node evidence value")
    }

    fn test_run(topology_ref: &str, node_refs: &[String], child_refs: &[String], decision: &str) -> IoValue {
        crate::nixos_vm::test_run_value(&crate::nixos_vm::NixosVmTestRunInput {
            decision,
            topology_ref,
            scenario: SCENARIO,
            fault_profile: "none",
            node_evidence_refs: node_refs,
            child_workflow_refs: child_refs,
            replay_status: "non-replayable-vm-observations",
            diagnostics: &[],
            log_refs: &[local_ref("vm-log")],
            caveats: &["vm evidence does not grant authority or policy trust".to_string()],
        })
        .expect("test run value")
    }

    fn fixture(decision: &str) -> (IoValue, Vec<IoValue>, IoValue, Vec<String>) {
        let topology = topology();
        let topology_ref = crate::preserves_rail::canonical_hash(&topology).expect("topology ref");
        let nodes = vec![node_evidence(NODE_A, "a"), node_evidence(NODE_B, "b")];
        let node_refs = canonical_refs(&nodes).expect("node refs");
        let child_refs = vec![local_ref("protocol"), local_ref("job"), local_ref("coordination")];
        let run = test_run(&topology_ref, &node_refs, &child_refs, decision);
        (topology, nodes, run, child_refs)
    }

    #[test]
    fn passing_vm_evidence_validates_semantically() {
        let (topology, nodes, run, child_refs) = fixture("pass");
        let validation = validate_nixos_vm_evidence(&NixosVmEvidenceValidationInput {
            topology_value: &topology,
            node_evidence_values: &nodes,
            test_run_value: &run,
            prod_soak_values: &[],
            expected_nodes: &[NODE_A.to_string(), NODE_B.to_string()],
            expected_package_ref: None,
            expected_child_refs: &child_refs,
        })
        .expect("VM validation");
        assert_eq!(validation.decision, "pass");
        assert!(validation.diagnostics.is_empty());
    }

    #[test]
    fn marker_only_or_wrong_topology_evidence_denies() {
        let (topology, nodes, _, child_refs) = fixture("pass");
        let wrong_topology_ref = local_ref("wrong-topology");
        let node_refs = canonical_refs(&nodes).expect("node refs");
        let run = test_run(&wrong_topology_ref, &node_refs, &child_refs, "pass");
        let validation = validate_nixos_vm_evidence(&NixosVmEvidenceValidationInput {
            topology_value: &topology,
            node_evidence_values: &nodes,
            test_run_value: &run,
            prod_soak_values: &[],
            expected_nodes: &[NODE_A.to_string(), NODE_B.to_string()],
            expected_package_ref: None,
            expected_child_refs: &child_refs,
        })
        .expect("VM validation");
        assert_eq!(validation.decision, "deny");
        assert!(validation.diagnostics.iter().any(|diagnostic| diagnostic == "test-run-topology-ref-mismatch"));
    }

    #[test]
    fn deny_receipt_cannot_be_overridden_by_logs() {
        let (topology, nodes, run, child_refs) = fixture("deny");
        let validation = validate_nixos_vm_evidence(&NixosVmEvidenceValidationInput {
            topology_value: &topology,
            node_evidence_values: &nodes,
            test_run_value: &run,
            prod_soak_values: &[],
            expected_nodes: &[NODE_A.to_string(), NODE_B.to_string()],
            expected_package_ref: None,
            expected_child_refs: &child_refs,
        })
        .expect("VM validation");
        assert_eq!(validation.decision, "deny");
        assert!(validation.diagnostics.iter().any(|diagnostic| diagnostic == "vm-test-run-not-pass"));
    }

    #[test]
    fn manifest_binds_authoritative_and_diagnostic_artifacts() {
        let entries = vec![
            VmEvidenceManifestEntry {
                path: "topology.preserves".to_string(),
                kind: "nixos-vm-topology".to_string(),
                content_ref: local_ref("topology"),
                diagnostic_only: false,
            },
            VmEvidenceManifestEntry {
                path: "run.txt".to_string(),
                kind: "log".to_string(),
                content_ref: local_ref("run-log"),
                diagnostic_only: true,
            },
        ];
        let manifest =
            vm_evidence_manifest_value(&entries, &["logs are diagnostic only".to_string()]).expect("manifest value");
        let rendered = crate::preserves_rail::to_text(&manifest).expect("render manifest");
        assert!(rendered.contains("nixos-vm-evidence-manifest-v1"));
        assert!(rendered.contains("diagnostic-only #t"));
    }

    #[test]
    fn duplicate_manifest_path_is_rejected() {
        let duplicate = VmEvidenceManifestEntry {
            path: "topology.preserves".to_string(),
            kind: "nixos-vm-topology".to_string(),
            content_ref: local_ref("topology"),
            diagnostic_only: false,
        };
        let error = vm_evidence_manifest_value(&[duplicate.clone(), duplicate], &[])
            .expect_err("duplicate manifest path must fail");
        assert!(error.to_string().contains("duplicate VM evidence manifest path"));
    }
}
