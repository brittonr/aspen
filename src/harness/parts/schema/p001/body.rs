
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectLogEntry {
    pub sequence: u64,
    pub request: IoValue,
    pub response: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyGateEvidence {
    pub value: IoValue,
    pub policy_ref: String,
    pub nickel_source_ref: String,
    pub nickel_export_ref: String,
    pub basalt_preflight_ref: String,
    pub checks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityGateEvidence {
    pub value: IoValue,
    pub capability_ref: String,
    pub authority_preflight_ref: String,
    pub proofset_ref: String,
    pub grant_refs: Vec<String>,
    pub ucan_verification_receipt_refs: Vec<String>,
    pub derived_grant_refs: Vec<String>,
    pub checks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetGateEvidence {
    pub value: IoValue,
    pub budget_ref: String,
    pub nickel_source_ref: String,
    pub nickel_export_ref: String,
    pub basalt_preflight_ref: String,
    pub checks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutorPreflightsEvidence {
    pub value: IoValue,
    pub preflights: Vec<ExecutorPreflightEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutorPreflightEvidence {
    pub value: IoValue,
    pub actor_id: String,
    pub kind: ActorKind,
    pub artifact_ref: Option<String>,
    pub sandbox_ref: String,
    pub allowed_hostcalls: Vec<String>,
    pub conformance_refs: Vec<String>,
    pub executor_receipts: Vec<IoValue>,
    pub steel_review: Option<SteelReviewReceipt>,
    pub wasm_inspection: Option<WasmInspectionReceipt>,
    pub checks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteelReviewReceipt {
    pub value: IoValue,
    pub source_ref: String,
    pub callable: String,
    pub allowed_hostcalls: Vec<String>,
    pub checks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmInspectionReceipt {
    pub value: IoValue,
    pub module_ref: String,
    pub module_kind: String,
    pub imports: Vec<WasmImportEvidence>,
    pub wit_ref: String,
    pub allowed_hostcalls: Vec<String>,
    pub checks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmImportEvidence {
    pub module: String,
    pub name: String,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    pub value: IoValue,
    pub observation_ref: String,
    pub index: u64,
    pub step_ref: String,
    pub before_state_hash: String,
    pub after_state_hash: String,
    pub event_refs: Vec<String>,
    pub events: Vec<IoValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionDecisionEvent {
    pub value: IoValue,
    pub request: super::core::AdmissionRequest,
    pub authority: Option<AdmissionAuthorityEvidence>,
    pub decision: crate::runtime::AdmissionDecision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionAuthorityEvidence {
    pub source: String,
    pub capability_ref: String,
    pub authorized: bool,
    pub grant_ref: Option<String>,
    pub request_ref: String,
    pub proofset_ref: String,
    pub ucan_verification_receipt_refs: Vec<String>,
    pub derived_grant_refs: Vec<String>,
    pub basalt_enforcement_receipt_ref: String,
    pub basalt_enforcement_receipt_value: IoValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventBoundary {
    EffectRequest,
    EffectResponse,
    PolicyDecision,
    ActorInput,
    HostcallRequest,
    HostcallDecision,
    ActorOutput,
    SteelExecution,
    WasmExecution,
    RuntimePredicate,
    Trace,
}

#[derive(Debug, Clone, Copy)]
pub struct HostcallEvidenceContext<'a> {
    pub sequence: u64,
    pub suite_ref: &'a str,
    pub step_ref: &'a str,
    pub policy_ref: &'a str,
    pub capability_ref: &'a str,
    pub budget_ref: &'a str,
}

#[derive(Default)]
struct SuiteFixtures {
    budget: Budget,
    has_budget_fixture: bool,
    actors: Option<Vec<ActorDecl>>,
    has_actor_fixture: bool,
    capabilities: crate::runtime::CapabilityContext,
    has_capability_fixture: bool,
    policy: crate::runtime::AdmissionPolicy,
    has_policy_fixture: bool,
}

enum SuiteFieldStatus {
    Applied,
    Unknown,
}

impl SuiteFixtures {
    fn apply_field(&mut self, field: &Value<IoValue>) -> Result<SuiteFieldStatus> {
        if value_has_record_label(field, "budget-v1") {
            self.apply_budget(field)?;
            return Ok(SuiteFieldStatus::Applied);
        }
        if value_has_record_label(field, "actor-registry-v1") {
            self.apply_actors(field)?;
            return Ok(SuiteFieldStatus::Applied);
        }
        if value_has_record_label(field, "capabilities-v1") {
            self.apply_capabilities(field)?;
            return Ok(SuiteFieldStatus::Applied);
        }
        if value_has_record_label(field, "policy-v1") {
            self.apply_policy(field)?;
            return Ok(SuiteFieldStatus::Applied);
        }
        Ok(SuiteFieldStatus::Unknown)
    }

    fn apply_budget(&mut self, field: &Value<IoValue>) -> Result<()> {
        if self.has_budget_fixture {
            return Err(MoltenError::invalid_harness("duplicate suite budget fixture"));
        }
        self.budget = parse_budget_limits(&value_to_iovalue(field))?;
        self.has_budget_fixture = true;
        Ok(())
    }

    fn apply_actors(&mut self, field: &Value<IoValue>) -> Result<()> {
        if self.actors.is_some() {
            return Err(MoltenError::invalid_harness("duplicate suite actor registry fixture"));
        }
        self.actors = Some(parse_actor_registry(&value_to_iovalue(field))?);
        self.has_actor_fixture = true;
        Ok(())
    }

    fn apply_capabilities(&mut self, field: &Value<IoValue>) -> Result<()> {
        if self.has_capability_fixture {
            return Err(MoltenError::invalid_harness("duplicate suite capability fixture"));
        }
        self.capabilities = parse_capabilities(&value_to_iovalue(field))?;
        self.has_capability_fixture = true;
        Ok(())
    }

    fn apply_policy(&mut self, field: &Value<IoValue>) -> Result<()> {
        if self.has_policy_fixture {
            return Err(MoltenError::invalid_harness("duplicate suite policy fixture"));
        }
        self.policy = parse_policy(&value_to_iovalue(field))?;
        self.has_policy_fixture = true;
        Ok(())
    }
}

pub fn parse_suite(value: &IoValue) -> Result<Suite> {
    let suite = value
        .collect_simple_record("harness-suite-v1", None)
        .ok_or_else(|| MoltenError::invalid_harness("expected <harness-suite-v1 ...>"))?;
    let arity = suite.fields_iter().count();
    if !(4..=8).contains(&arity) {
        return Err(MoltenError::invalid_harness(format!(
            "expected <harness-suite-v1 ...> with arity 4 through 8, got {arity}"
        )));
    }
    let schema = required_string(&suite[0], "suite schema")?;
    if schema != crate::preserves_rail::HARNESS_SUITE_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported suite schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_SUITE_SCHEMA
        )));
    }
    let name = required_string(&suite[1], "suite name")?;
    let seed = required_u64(&suite[2], "suite seed")?;
    let (cursor, fixtures) = suite_fixtures(&suite, arity)?;
    let step_values = required_sequence(&suite[cursor], "suite steps")?;
    let mut steps = Vec::with_capacity(step_values.len());
    for step in step_values.iter() {
        steps.push(parse_step(&step)?);
    }
    let actors = fixtures.actors.unwrap_or_else(|| infer_actor_registry(&steps));
    Ok(Suite {
        name,
        seed,
        budget: fixtures.budget,
        budget_explicit: fixtures.has_budget_fixture,
        actors,
        actors_explicit: fixtures.has_actor_fixture,
        capabilities: fixtures.capabilities,
        capabilities_explicit: fixtures.has_capability_fixture,
        policy: fixtures.policy,
        steps,
        source_value: value.clone(),
    })
}

fn suite_fixtures(suite: &Record<Value<IoValue>>, arity: usize) -> Result<(usize, SuiteFixtures)> {
    let mut cursor = 3;
    let mut fixtures = SuiteFixtures::default();
    while cursor < arity - 1 {
        match fixtures.apply_field(&suite[cursor])? {
            SuiteFieldStatus::Applied => cursor += 1,
            SuiteFieldStatus::Unknown => {
                return Err(MoltenError::invalid_harness(
                    "unexpected suite field before steps; expected optional budget, actor registry, capabilities, policy, then steps",
                ));
            }
        }
    }
    Ok((cursor, fixtures))
}

pub fn suite_ref(suite: &Suite) -> Result<String> {
    canonical_hash(&suite.source_value)
}

pub fn step_value(step: &super::core::CoreStep) -> IoValue {
    match step {
        super::core::CoreStep::Send { from, to, body } => {
            record("send", vec![string(from), string(to), body.as_iovalue().clone()])
        }
        super::core::CoreStep::Observe { actor, pattern } => {
            record("observe", vec![string(actor), pattern.as_iovalue().clone()])
        }
        super::core::CoreStep::Assert { actor, value } => {
            record("assert", vec![string(actor), value.as_iovalue().clone()])
        }
        super::core::CoreStep::Retract { actor, value } => {
            record("retract", vec![string(actor), value.as_iovalue().clone()])
        }
        super::core::CoreStep::Clock { actor } => record("clock", vec![string(actor)]),
        super::core::CoreStep::Random { actor, upper } => record("random", vec![string(actor), u64_value(*upper)]),
    }
}
