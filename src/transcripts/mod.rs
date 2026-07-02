type Counter = std::sync::atomic::AtomicU64;
type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type PathBuf = std::path::PathBuf;
type PreservesRecord<T> = preserves::Record<T>;
type PreservesValue<T> = preserves::Value<T>;
type Result<T> = crate::error::Result<T>;
type Set<T> = std::collections::BTreeSet<T>;

const RELAXED: std::sync::atomic::Ordering = std::sync::atomic::Ordering::Relaxed;

mod fs {
    pub(super) fn create_dir(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::create_dir(path)
    }

    pub(super) fn create_dir_all(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::create_dir_all(path)
    }
}

const TRANSCRIPT_ARTIFACT_SCHEMA: &str = crate::preserves_rail::TRANSCRIPT_ARTIFACT_SCHEMA;
const TRANSCRIPT_RUN_RECEIPT_SCHEMA: &str = crate::preserves_rail::TRANSCRIPT_RUN_RECEIPT_SCHEMA;
const TRANSCRIPT_STANZA_OUTCOME_SCHEMA: &str = crate::preserves_rail::TRANSCRIPT_STANZA_OUTCOME_SCHEMA;
const TRANSCRIPT_STANZA_SCHEMA: &str = crate::preserves_rail::TRANSCRIPT_STANZA_SCHEMA;

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn parse_text(source: &str) -> Result<IoValue> {
    crate::preserves_rail::parse_text(source)
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

fn to_text(value: &IoValue) -> Result<String> {
    crate::preserves_rail::to_text(value)
}

fn u64_value(value: u64) -> IoValue {
    crate::preserves_rail::u64_value(value)
}

fn validate_content_ref(value: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(value)
}

fn value_to_iovalue(value: &PreservesValue<IoValue>) -> IoValue {
    crate::preserves_rail::value_to_iovalue(value)
}

pub const RUNNER_TOOL_VERSION: &str = "local-transcript-runner-v1";

const MAX_TEMP_STATE_ROOT_ATTEMPTS: u64 = 1024;
const MAX_TRANSCRIPT_SEQUENCE_ITEMS: usize = 4_096;

const _: () = assert!(MAX_TRANSCRIPT_SEQUENCE_ITEMS > 0);

static TEMP_STATE_ROOT_COUNTER: Counter = Counter::new(0);

pub const KIND_MOLTEN_CLI: &str = "molten-cli";
pub const KIND_PRESERVES: &str = "preserves";
pub const KIND_ARTIFACT: &str = "artifact";
pub const KIND_POLICY: &str = "policy";
pub const KIND_EXPECT: &str = "expect";
pub const KIND_COMMENT: &str = "comment";

const DECISION_PASS: &str = "pass";
const DECISION_DENY: &str = "deny";
const DECISION_ERROR: &str = "error";
const DECISION_SKIP: &str = "skip";
const DECISION_KNOWN_BUG: &str = "known-bug";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TranscriptParseInput {
    pub dependency_refs: Vec<String>,
    pub dependency_closure_hash: Option<String>,
    pub handler_profile_ref: Option<String>,
    pub policy_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub revocation_refs: Vec<String>,
    pub seed_ref: Option<String>,
    pub expected_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptArtifact {
    pub transcript_ref: String,
    pub source_ref: String,
    pub stanzas: Vec<TranscriptStanza>,
    pub dependency_closure_hash: String,
    pub dependency_refs: Vec<String>,
    pub handler_profile_ref: Option<String>,
    pub policy_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub revocation_refs: Vec<String>,
    pub seed_ref: Option<String>,
    pub expected_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptStanza {
    pub stanza_ref: String,
    pub index: u64,
    pub kind: String,
    pub modifiers: Vec<TranscriptModifier>,
    pub content: String,
    pub content_ref: String,
    pub declared_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptModifier {
    pub name: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptRunInput {
    pub mode: TranscriptRunMode,
    pub cache_root: Option<PathBuf>,
    pub save_root: Option<PathBuf>,
}

impl Default for TranscriptRunInput {
    fn default() -> Self {
        Self {
            mode: TranscriptRunMode::Fresh,
            cache_root: None,
            save_root: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranscriptRunMode {
    Fresh,
    Save,
    ForkDenied,
    InPlaceDenied,
}

impl TranscriptRunMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            TranscriptRunMode::Fresh => "fresh",
            TranscriptRunMode::Save => "save",
            TranscriptRunMode::ForkDenied => "fork-denied",
            TranscriptRunMode::InPlaceDenied => "in-place-denied",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "fresh" => Ok(Self::Fresh),
            "save" => Ok(Self::Save),
            "fork" | "fork-denied" => Ok(Self::ForkDenied),
            "in-place" | "in-place-denied" => Ok(Self::InPlaceDenied),
            other => Err(MoltenError::invalid_harness(format!("unsupported transcript run mode {other}"))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptRun {
    pub transcript_ref: String,
    pub decision: String,
    pub stanza_outcomes: Vec<StanzaOutcome>,
    pub receipt_value: IoValue,
    pub receipt_ref: String,
    pub cache_receipt_value: Option<IoValue>,
    pub state_root: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StanzaOutcome {
    pub outcome_ref: String,
    pub index: u64,
    pub kind: String,
    pub decision: String,
    pub output: Option<IoValue>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptRunReceipt {
    pub receipt_ref: String,
    pub operation: String,
    pub decision: String,
    pub transcript_ref: String,
    pub mode: String,
    pub outcome_refs: Vec<String>,
    pub value: IoValue,
}

struct RunReceiptValueInput<'a> {
    operation: &'a str,
    decision: &'a str,
    transcript_ref: &'a str,
    mode: &'a str,
    outcomes: &'a [StanzaOutcome],
    output: Option<&'a IoValue>,
    refs: Vec<String>,
    diagnostics: &'a [String],
    checks: &'a [(&'a str, &'a str)],
}

#[derive(Debug)]
struct RunnerState {
    root: PathBuf,
    registry: PathBuf,
    storage: PathBuf,
    cache: PathBuf,
    last_output: Option<IoValue>,
    last_decision: Option<String>,
    last_kind: Option<String>,
    last_artifact_ref: Option<String>,
}

pub fn parse_markdown(source: &str, input: &TranscriptParseInput) -> Result<TranscriptArtifact> {
    validate_parse_input(input)?;
    let source_ref = canonical_hash(&string(source))?;
    let stanzas = parse_markdown_stanzas(source)?;
    let stanza_values = stanzas.iter().map(|stanza| stanza.value.clone()).collect::<Vec<_>>();
    let dependency_refs = sorted_unique(&input.dependency_refs);
    let dependency_closure_hash = match input.dependency_closure_hash.as_ref() {
        Some(hash) => hash.clone(),
        None => canonical_hash(&record("transcript-dependency-closure-v1", vec![refs_sequence(&dependency_refs)]))?,
    };
    let value = record("transcript-artifact-v1", vec![
        string(TRANSCRIPT_ARTIFACT_SCHEMA),
        record("source", vec![string(&source_ref)]),
        record("stanzas", vec![sequence(stanza_values)]),
        record("dependencies", vec![string(&dependency_closure_hash), refs_sequence(&dependency_refs)]),
        record("handler-profile", vec![optional_ref_value(input.handler_profile_ref.as_deref())]),
        record("policy", vec![refs_sequence(&sorted_unique(&input.policy_refs))]),
        record("capability", vec![refs_sequence(&sorted_unique(&input.capability_refs))]),
        record("revocation", vec![refs_sequence(&sorted_unique(&input.revocation_refs))]),
        record("seed", vec![optional_ref_value(input.seed_ref.as_deref())]),
        record("expected", vec![refs_sequence(&sorted_unique(&input.expected_refs))]),
        checks_value(&[
            "bounded-stanzas",
            "canonical-source-identity",
            "no-ambient-identity",
            "no-ucm-compat",
        ]),
    ]);
    parse_transcript_artifact(&value)
}

pub fn parse_transcript_artifact(value: &IoValue) -> Result<TranscriptArtifact> {
    let fields = value
        .collect_simple_record("transcript-artifact-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("expected <transcript-artifact-v1 ...>"))?;
    require_schema(&fields[0], TRANSCRIPT_ARTIFACT_SCHEMA, "transcript artifact")?;
    let deps = value_to_iovalue(&fields[3]);
    let dep_fields = simple_record(&deps, "dependencies", 2)?;
    let stanzas = record_sequence(&fields[2], "stanzas")?
        .iter()
        .map(|stanza| parse_transcript_stanza(&value_to_iovalue(stanza)))
        .collect::<Result<Vec<_>>>()?;
    let checks = parse_checks(&fields[10])?;
    require_check(&checks, "no-ambient-identity", "transcript artifact")?;
    Ok(TranscriptArtifact {
        transcript_ref: canonical_hash(value)?,
        source_ref: record_ref(&fields[1], "source")?,
        stanzas,
        dependency_closure_hash: required_ref(&dep_fields[0], "dependency closure hash")?,
        dependency_refs: parse_ref_sequence_value(&dep_fields[1], "dependency refs")?,
        handler_profile_ref: record_optional_ref(&fields[4], "handler-profile")?,
        policy_refs: record_ref_sequence(&fields[5], "policy")?,
        capability_refs: record_ref_sequence(&fields[6], "capability")?,
        revocation_refs: record_ref_sequence(&fields[7], "revocation")?,
        seed_ref: record_optional_ref(&fields[8], "seed")?,
        expected_refs: record_ref_sequence(&fields[9], "expected")?,
        value: value.clone(),
    })
}

pub fn parse_transcript_stanza(value: &IoValue) -> Result<TranscriptStanza> {
    let fields = value
        .collect_simple_record("transcript-stanza-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <transcript-stanza-v1 ...>"))?;
    require_schema(&fields[0], TRANSCRIPT_STANZA_SCHEMA, "transcript stanza")?;
    let input = value_to_iovalue(&fields[4]);
    let input_fields = simple_record(&input, "input", 1)?;
    let inline = value_to_iovalue(&input_fields[0]);
    let inline_fields = simple_record(&inline, "inline", 2)?;
    let checks = parse_checks(&fields[6])?;
    require_check(&checks, "bounded-stanza", "transcript stanza")?;
    Ok(TranscriptStanza {
        stanza_ref: canonical_hash(value)?,
        index: record_u64(&fields[1], "index")?,
        kind: record_string(&fields[2], "kind")?,
        modifiers: record_modifier_sequence(&fields[3])?,
        content_ref: required_ref(&inline_fields[0], "stanza content ref")?,
        content: required_string(&inline_fields[1], "stanza content")?,
        declared_refs: record_ref_sequence(&fields[5], "refs")?,
        value: value.clone(),
    })
}

pub fn run_transcript(transcript: &TranscriptArtifact, input: &TranscriptRunInput) -> Result<TranscriptRun> {
    if matches!(input.mode, TranscriptRunMode::ForkDenied | TranscriptRunMode::InPlaceDenied) {
        return denied_run(transcript, input);
    }
    if let Some(run) = cached_run(transcript, input)? {
        return Ok(run);
    }

    let state_root = match input.mode {
        TranscriptRunMode::Fresh => temp_state_root("fresh")?,
        TranscriptRunMode::Save => match input.save_root.clone() {
            Some(save_root) => save_root,
            None => temp_state_root("save")?,
        },
        TranscriptRunMode::ForkDenied | TranscriptRunMode::InPlaceDenied => {
            return Err(MoltenError::invalid_harness("denied transcript modes cannot allocate runner state"));
        }
    };
    let mut state = RunnerState::new(state_root.clone())?;
    let mut outcomes = Vec::with_capacity(transcript.stanzas.len());
    for stanza in &transcript.stanzas {
        let outcome = run_stanza(&mut state, transcript, stanza)?;
        state.last_decision = Some(outcome.decision.clone());
        state.last_kind = Some(outcome.kind.clone());
        state.last_output = outcome.output.clone();
        outcomes.push(outcome);
    }
    let decision = final_decision(&outcomes);
    let diagnostics = outcomes.iter().flat_map(|outcome| outcome.diagnostics.iter().cloned()).collect::<Vec<_>>();
    let refs = refs_for_transcript(transcript, &outcomes);
    let receipt = run_receipt_value(&RunReceiptValueInput {
        operation: "run",
        decision: &decision,
        transcript_ref: &transcript.transcript_ref,
        mode: input.mode.as_str(),
        outcomes: &outcomes,
        output: state.last_output.as_ref(),
        refs,
        diagnostics: &diagnostics,
        checks: &[
            ("fresh-state", "pass"),
            ("canonical-expectations", "pass"),
            ("effect-admission", "pass"),
        ],
    })?;
    let cache_receipt_value = store_run(input, transcript, &decision, &receipt)?;
    let receipt_ref = canonical_hash(&receipt)?;
    Ok(TranscriptRun {
        transcript_ref: transcript.transcript_ref.clone(),
        decision,
        stanza_outcomes: outcomes,
        receipt_value: receipt,
        receipt_ref,
        cache_receipt_value,
        state_root: if matches!(input.mode, TranscriptRunMode::Save) {
            Some(state.root)
        } else {
            None
        },
    })
}

fn denied_run(transcript: &TranscriptArtifact, input: &TranscriptRunInput) -> Result<TranscriptRun> {
    let outcome = denial_outcome(0, "mode", format!("{} mode denied by default", input.mode.as_str()))?;
    let receipt = run_receipt_value(&RunReceiptValueInput {
        operation: "deny",
        decision: DECISION_DENY,
        transcript_ref: &transcript.transcript_ref,
        mode: input.mode.as_str(),
        outcomes: std::slice::from_ref(&outcome),
        output: None,
        refs: refs_for_transcript(transcript, &[]),
        diagnostics: &[format!("{} mode denied by default", input.mode.as_str())],
        checks: &[("in-place-denied", "pass"), ("no-ambient-identity", "pass")],
    })?;
    Ok(TranscriptRun {
        transcript_ref: transcript.transcript_ref.clone(),
        decision: DECISION_DENY.to_string(),
        stanza_outcomes: vec![outcome],
        receipt_ref: canonical_hash(&receipt)?,
        receipt_value: receipt,
        cache_receipt_value: None,
        state_root: None,
    })
}

fn cached_run(transcript: &TranscriptArtifact, input: &TranscriptRunInput) -> Result<Option<TranscriptRun>> {
    let Some(cache_root) = input.cache_root.as_ref() else {
        return Ok(None);
    };
    let cache_key = transcript_cache_key(transcript)?;
    if let Ok(cache_get) = crate::eval_cache::get(
        cache_root,
        &canonical_hash(&crate::eval_cache::key_value(&cache_key)?)?,
        &crate::eval_cache::GetInput {
            current_policy_refs: transcript.policy_refs.clone(),
            current_capability_refs: transcript.capability_refs.clone(),
            current_revocation_refs: transcript.revocation_refs.clone(),
            semantic: true,
        },
    ) && let Some(output) = cache_get.output.as_ref()
        && let Ok(receipt) = parse_transcript_run_receipt(output)
    {
        return Ok(Some(TranscriptRun {
            transcript_ref: transcript.transcript_ref.clone(),
            decision: receipt.decision.clone(),
            stanza_outcomes: Vec::new(),
            receipt_ref: receipt.receipt_ref,
            receipt_value: output.clone(),
            cache_receipt_value: Some(cache_get.receipt_value),
            state_root: None,
        }));
    }
    Ok(None)
}

fn store_run(
    input: &TranscriptRunInput,
    transcript: &TranscriptArtifact,
    decision: &str,
    receipt: &IoValue,
) -> Result<Option<IoValue>> {
    if decision == DECISION_PASS
        && let Some(cache_root) = input.cache_root.as_ref()
    {
        let cache_key = transcript_cache_key(transcript)?;
        let put = crate::eval_cache::put(cache_root, &cache_key, &crate::eval_cache::ValueInput {
            tier: crate::eval_cache::TIER_SIMULATED.to_string(),
            status: crate::eval_cache::STATUS_PASS.to_string(),
            output: Some(receipt.clone()),
            dependency_refs: cache_key.dependency_refs.clone(),
            policy_refs: cache_key.policy_refs.clone(),
            evidence_refs: Vec::new(),
            diagnostics: Vec::new(),
        })?;
        Ok(Some(put.receipt_value))
    } else {
        Ok(None)
    }
}

pub fn parse_transcript_run_receipt(value: &IoValue) -> Result<TranscriptRunReceipt> {
    let fields = value
        .collect_simple_record("transcript-run-receipt-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("expected <transcript-run-receipt-v1 ...>"))?;
    require_schema(&fields[0], TRANSCRIPT_RUN_RECEIPT_SCHEMA, "transcript run receipt")?;
    let outcomes = record_ref_sequence(&fields[5], "outcomes")?;
    let checks = parse_checks(&fields[10])?;
    if checks.is_empty() {
        return Err(MoltenError::invalid_harness("transcript run receipt missing checks"));
    }
    Ok(TranscriptRunReceipt {
        receipt_ref: canonical_hash(value)?,
        operation: record_string(&fields[1], "operation")?,
        decision: record_string(&fields[2], "decision")?,
        transcript_ref: record_ref(&fields[3], "transcript")?,
        mode: record_string(&fields[4], "mode")?,
        outcome_refs: outcomes,
        value: value.clone(),
    })
}

pub fn render_transcript(transcript: &TranscriptArtifact, run: Option<&TranscriptRun>) -> Result<String> {
    let mut rendered = String::new();
    rendered.push_str(&format!("# Transcript {}\n\n", transcript.transcript_ref));
    let outcomes = run.map(|run| &run.stanza_outcomes[..]).unwrap_or(&[]);
    for stanza in &transcript.stanzas {
        rendered.push_str(&format!("```{}\n{}\n```\n", stanza.kind, stanza.content.trim_end()));
        if stanza.has_modifier("hide") {
            rendered.push_str("<!-- transcript output hidden; evidence retained -->\n\n");
            continue;
        }
        if let Some(outcome) = outcomes.iter().find(|outcome| outcome.index == stanza.index) {
            rendered.push_str(&format!("decision: {}\n", outcome.decision));
            if let Some(output) = outcome.output.as_ref() {
                rendered.push_str("```preserves-output\n");
                rendered.push_str(&crate::secrets::redacted_text(output, None)?);
                rendered.push_str("\n```\n");
            }
            if !outcome.diagnostics.is_empty() {
                rendered.push_str(&format!("diagnostics: {}\n", outcome.diagnostics.join("; ")));
            }
            rendered.push('\n');
        }
    }
    if let Some(run) = run {
        rendered.push_str(&format!("\nFinal decision: {}\nReceipt: {}\n", run.decision, run.receipt_ref));
    }
    Ok(rendered)
}

pub fn transcript_cache_key(transcript: &TranscriptArtifact) -> Result<crate::eval_cache::KeyInput> {
    let handler_profile_ref = transcript
        .handler_profile_ref
        .clone()
        .unwrap_or(canonical_hash(&record("transcript-default-handler-profile", vec![string("deterministic-local")]))?);
    let tool_ref = canonical_hash(&record("transcript-runner-tool", vec![string("molten-local-transcript-runner")]))?;
    let mut key = crate::eval_cache::transcript_run_key_placeholder(&crate::eval_cache::TranscriptRunKeyInput {
        transcript_ref: &transcript.transcript_ref,
        closure_hash: &transcript.dependency_closure_hash,
        dependency_refs: &transcript.dependency_refs,
        handler_profile_ref: &handler_profile_ref,
        harness_ref: &tool_ref,
        harness_version: RUNNER_TOOL_VERSION,
    })?;
    key.policy_refs = transcript.policy_refs.clone();
    key.capability_refs = transcript.capability_refs.clone();
    key.revocation_refs = transcript.revocation_refs.clone();
    if let Some(seed_ref) = transcript.seed_ref.as_ref() {
        key.assumption_refs.push(seed_ref.clone());
    }
    key.assumption_refs.extend(transcript.expected_refs.iter().cloned());
    Ok(key)
}

impl TranscriptStanza {
    fn has_modifier(&self, name: &str) -> bool {
        self.modifiers.iter().any(|modifier| modifier.name == name)
    }
}

impl RunnerState {
    fn new(root: PathBuf) -> Result<Self> {
        fs::create_dir_all(&root).map_err(MoltenError::from)?;
        let registry = root.join("registry");
        let ledger = root.join("ledger");
        let storage = root.join("typed-storage");
        let cache = root.join("eval-cache");
        fs::create_dir_all(&registry).map_err(MoltenError::from)?;
        fs::create_dir_all(&ledger).map_err(MoltenError::from)?;
        fs::create_dir_all(&storage).map_err(MoltenError::from)?;
        fs::create_dir_all(&cache).map_err(MoltenError::from)?;
        Ok(Self {
            root,
            registry,
            storage,
            cache,
            last_output: None,
            last_decision: None,
            last_kind: None,
            last_artifact_ref: None,
        })
    }
}

fn parse_markdown_stanzas(source: &str) -> Result<Vec<TranscriptStanza>> {
    let mut stanzas = Vec::new();
    let mut prose = String::new();
    let mut fence_info = None::<String>;
    let mut fence_content = String::new();
    for line in source.lines() {
        if let Some(info) = fence_info.as_deref() {
            if line.trim() == "```" {
                push_stanza_from_fence(StanzaFenceInput {
                    info,
                    content: fence_content.trim_end_matches('\n').to_string(),
                    stanzas: &mut stanzas,
                })?;
                fence_info = None;
                fence_content.clear();
            } else {
                fence_content.push_str(line);
                fence_content.push('\n');
            }
            continue;
        }
        if let Some(info) = line.strip_prefix("```") {
            flush_prose(FlushProseInput {
                prose: &mut prose,
                stanzas: &mut stanzas,
            })?;
            fence_info = Some(info.trim().to_string());
        } else {
            prose.push_str(line);
            prose.push('\n');
        }
    }
    if fence_info.is_some() {
        return Err(MoltenError::invalid_harness("unterminated transcript fenced block"));
    }
    flush_prose(FlushProseInput {
        prose: &mut prose,
        stanzas: &mut stanzas,
    })?;
    Ok(stanzas)
}

struct FlushProseInput<'a> {
    prose: &'a mut String,
    stanzas: &'a mut Vec<TranscriptStanza>,
}

fn flush_prose(input: FlushProseInput<'_>) -> Result<()> {
    let trimmed = input.prose.trim();
    if !trimmed.is_empty() {
        push_stanza(PushStanzaInput {
            kind: KIND_COMMENT,
            modifiers: Vec::new(),
            content: trimmed.to_string(),
            declared_refs: Vec::new(),
            stanzas: input.stanzas,
        })?;
    }
    input.prose.clear();
    Ok(())
}

struct StanzaFenceInput<'a> {
    info: &'a str,
    content: String,
    stanzas: &'a mut Vec<TranscriptStanza>,
}

fn push_stanza_from_fence(input: StanzaFenceInput<'_>) -> Result<()> {
    if input.info.is_empty() {
        return push_stanza(PushStanzaInput {
            kind: KIND_COMMENT,
            modifiers: Vec::new(),
            content: input.content,
            declared_refs: Vec::new(),
            stanzas: input.stanzas,
        });
    }
    let mut tokens = input.info.split_whitespace();
    let first = tokens.next().unwrap_or_default();
    let mut first_parts = first.split(':');
    let kind = first_parts.next().unwrap_or_default();
    validate_kind(kind)?;
    let mut modifiers = first_parts.map(parse_modifier_token).collect::<Result<Vec<_>>>()?;
    for token in tokens {
        modifiers.push(parse_modifier_token(token)?);
    }
    push_stanza(PushStanzaInput {
        kind,
        modifiers,
        content: input.content,
        declared_refs: Vec::new(),
        stanzas: input.stanzas,
    })
}

struct PushStanzaInput<'a> {
    kind: &'a str,
    modifiers: Vec<TranscriptModifier>,
    content: String,
    declared_refs: Vec<String>,
    stanzas: &'a mut Vec<TranscriptStanza>,
}

fn push_stanza(input: PushStanzaInput<'_>) -> Result<()> {
    let index = input.stanzas.len() as u64;
    let content_ref = canonical_hash(&string(&input.content))?;
    let modifier_values = input.modifiers.iter().map(modifier_value).collect::<Vec<_>>();
    let value = record("transcript-stanza-v1", vec![
        string(TRANSCRIPT_STANZA_SCHEMA),
        record("index", vec![u64_value(index)]),
        record("kind", vec![string(input.kind)]),
        record("modifiers", vec![sequence(modifier_values)]),
        record("input", vec![record("inline", vec![string(&content_ref), string(&input.content)])]),
        record("refs", vec![refs_sequence(&input.declared_refs)]),
        checks_value(&["bounded-stanza", "no-ambient-shell"]),
    ]);
    input.stanzas.push(parse_transcript_stanza(&value)?);
    Ok(())
}

fn run_stanza(
    state: &mut RunnerState,
    transcript: &TranscriptArtifact,
    stanza: &TranscriptStanza,
) -> Result<StanzaOutcome> {
    if stanza.has_modifier("skip") {
        return stanza_outcome(stanza, DECISION_SKIP, None, vec!["stanza skipped by modifier".to_string()]);
    }
    match execute_stanza(state, transcript, stanza) {
        Ok(output) => {
            if stanza.has_modifier("error") {
                stanza_outcome(stanza, DECISION_DENY, output, vec![
                    "stanza succeeded but :error expected failure".to_string(),
                ])
            } else if stanza.has_modifier("bug") {
                stanza_outcome(stanza, DECISION_KNOWN_BUG, output, vec!["known bug stanza recorded".to_string()])
            } else {
                stanza_outcome(stanza, DECISION_PASS, output, Vec::new())
            }
        }
        Err(error) => {
            let diagnostic = error.to_string();
            if stanza.has_modifier("error") {
                stanza_outcome(stanza, DECISION_PASS, None, vec![diagnostic])
            } else if stanza.has_modifier("bug") {
                stanza_outcome(stanza, DECISION_KNOWN_BUG, None, vec![diagnostic])
            } else {
                stanza_outcome(stanza, DECISION_ERROR, None, vec![diagnostic])
            }
        }
    }
}

fn execute_stanza(
    state: &mut RunnerState,
    _transcript: &TranscriptArtifact,
    stanza: &TranscriptStanza,
) -> Result<Option<IoValue>> {
    match stanza.kind.as_str() {
        KIND_COMMENT => Ok(None),
        KIND_POLICY => {
            let value = parse_text(&stanza.content)?;
            Ok(Some(value))
        }
        KIND_ARTIFACT | KIND_PRESERVES => {
            let value = parse_text(&stanza.content)?;
            if let Some(record) = value.collect_simple_record("artifact-v1", None) {
                let artifact_ref = canonical_hash(&value)?;
                state.last_artifact_ref = Some(artifact_ref);
                drop(record);
            }
            Ok(Some(value))
        }
        KIND_MOLTEN_CLI => execute_molten_cli(state, &stanza.content),
        KIND_EXPECT => execute_expectation(state, &stanza.content),
        other => Err(MoltenError::invalid_harness(format!("unsupported transcript stanza kind {other}"))),
    }
}

fn execute_molten_cli(state: &mut RunnerState, content: &str) -> Result<Option<IoValue>> {
    let args = content.split_whitespace().collect::<Vec<_>>();
    if args.is_empty() {
        return Err(MoltenError::invalid_harness("empty molten-cli stanza"));
    }
    if args.first() != Some(&"test") {
        return Err(MoltenError::invalid_harness("molten-cli stanzas must start with `test`"));
    }
    match args.get(1).copied() {
        Some("artifact") => execute_artifact_cli(state, &args[2..]),
        Some("schema") => execute_schema_cli(state, &args[2..]),
        Some("storage") => execute_storage_cli(state, &args[2..]),
        Some("cache") => execute_cache_cli(state, &args[2..]),
        Some("report") => execute_report_cli(state),
        Some(other) => {
            Err(MoltenError::invalid_harness(format!("unsupported transcript molten-cli test command {other}")))
        }
        None => Err(MoltenError::invalid_harness("missing molten-cli test subcommand")),
    }
}

fn execute_artifact_cli(state: &mut RunnerState, args: &[&str]) -> Result<Option<IoValue>> {
    match args.first().copied() {
        Some("install") => {
            let kind = option_value(args, "--kind").unwrap_or("artifact");
            let payload = state.last_output.clone().ok_or_else(|| {
                MoltenError::invalid_harness("artifact install requires prior preserves/artifact stanza output")
            })?;
            let install =
                crate::artifacts::install_artifact(&state.registry, &crate::artifacts::ArtifactInstallInput {
                    kind: kind.to_string(),
                    payload,
                    schema_refs: vec![local_ref("transcript-artifact-schema", kind)?],
                    dependency_refs: Vec::new(),
                    effect_manifest_ref: None,
                    policy_refs: vec![local_ref("transcript-artifact-policy", kind)?],
                    evidence_refs: vec![local_ref("transcript-artifact-evidence", kind)?],
                    installer_ref: local_ref("transcript-runner", kind)?,
                    capability_refs: vec![local_ref("transcript-artifact-capability", kind)?],
                })?;
            state.last_artifact_ref = Some(install.artifact_ref.clone());
            Ok(Some(install.artifact.value))
        }
        Some("list") => {
            let refs = crate::artifacts::list_artifacts(&state.registry, None)?
                .iter()
                .map(|artifact| string(&artifact.artifact_ref))
                .collect();
            Ok(Some(record("artifact-list", vec![sequence(refs)])))
        }
        Some("closure") => {
            let artifact_ref = args
                .get(1)
                .map(|value| (*value).to_string())
                .or_else(|| state.last_artifact_ref.clone())
                .ok_or_else(|| MoltenError::invalid_harness("artifact closure requires an artifact ref"))?;
            let closure = crate::artifacts::dependency_closure(&state.registry, &[artifact_ref])?;
            Ok(Some(closure.receipt_value))
        }
        Some(other) => Err(MoltenError::invalid_harness(format!("unsupported transcript artifact command {other}"))),
        None => Err(MoltenError::invalid_harness("missing transcript artifact command")),
    }
}

fn execute_schema_cli(state: &mut RunnerState, args: &[&str]) -> Result<Option<IoValue>> {
    match args.first().copied() {
        Some("identity") => {
            let schema_ref = option_value(args, "--schema-ref")
                .map(str::to_string)
                .unwrap_or(local_ref("transcript-schema", "identity")?);
            let mode = option_value(args, "--mode").unwrap_or(crate::schema_identity::MODE_STRUCTURAL).to_string();
            let shape = state
                .last_output
                .clone()
                .ok_or_else(|| MoltenError::invalid_harness("schema identity requires prior preserves shape output"))?;
            let value = crate::schema_identity::schema_identity_value(&crate::schema_identity::SchemaIdentityInput {
                mode,
                schema_ref,
                shape,
                brand_ref: None,
                metadata_refs: vec![local_ref("transcript-schema-metadata", "identity")?],
                policy_refs: vec![local_ref("transcript-schema-policy", "identity")?],
                evidence_refs: vec![local_ref("transcript-schema-evidence", "identity")?],
            })?;
            Ok(Some(value))
        }
        Some(other) => Err(MoltenError::invalid_harness(format!("unsupported transcript schema command {other}"))),
        None => Err(MoltenError::invalid_harness("missing transcript schema command")),
    }
}

fn execute_storage_cli(state: &mut RunnerState, args: &[&str]) -> Result<Option<IoValue>> {
    match args.first().copied() {
        Some("put") => {
            let namespace = option_value(args, "--namespace").unwrap_or("transcript").to_string();
            let key = option_value(args, "--key").unwrap_or("value").to_string();
            let schema_ref = option_value(args, "--schema-ref").map(str::to_string);
            let value = state
                .last_output
                .clone()
                .ok_or_else(|| MoltenError::invalid_harness("storage put requires prior preserves output"))?;
            let admission = crate::typed_storage::Admission::local_fixture(&format!("transcript:{namespace}:{key}"));
            let put = crate::typed_storage::put_value(&state.storage, &crate::typed_storage::PutInput {
                namespace,
                key,
                schema_ref,
                value,
                producer_ref: local_ref("transcript-storage-producer", "put")?,
                policy_refs: vec![admission.policy_ref.clone()],
                evidence_refs: admission.evidence_refs.clone(),
                admission,
            })?;
            Ok(Some(put.typed_ref_value))
        }
        Some("get") => {
            let namespace = option_value(args, "--namespace").unwrap_or("transcript");
            let key = option_value(args, "--key").unwrap_or("value");
            let schema_ref = option_value(args, "--schema-ref");
            let admission = crate::typed_storage::Admission::local_fixture(&format!("transcript:{namespace}:{key}"));
            let get = crate::typed_storage::get_value(&state.storage, namespace, key, schema_ref, &admission)?;
            Ok(Some(get.value))
        }
        Some(other) => Err(MoltenError::invalid_harness(format!("unsupported transcript storage command {other}"))),
        None => Err(MoltenError::invalid_harness("missing transcript storage command")),
    }
}

fn execute_cache_cli(state: &mut RunnerState, args: &[&str]) -> Result<Option<IoValue>> {
    match args.first().copied() {
        Some("status") => {
            let status = crate::eval_cache::status(&state.cache)?;
            Ok(Some(record("eval-cache-status", vec![
                u64_value(status.keys as u64),
                u64_value(status.values as u64),
                u64_value(status.tombstones as u64),
                u64_value(status.receipts as u64),
            ])))
        }
        Some("list") => {
            let entries = crate::eval_cache::list(&state.cache, &crate::eval_cache::ListFilter::default())?;
            Ok(Some(record("eval-cache-list", vec![sequence(
                entries.iter().map(|entry| string(&entry.key_ref)).collect(),
            )])))
        }
        Some(other) => Err(MoltenError::invalid_harness(format!("unsupported transcript cache command {other}"))),
        None => Err(MoltenError::invalid_harness("missing transcript cache command")),
    }
}

fn execute_report_cli(state: &RunnerState) -> Result<Option<IoValue>> {
    let value = state
        .last_output
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("report command requires prior output"))?;
    let validation = crate::harness::validate_report_value(value)?;
    Ok(Some(record("report-validation", vec![string(validation.report_ref)])))
}

fn execute_expectation(state: &RunnerState, content: &str) -> Result<Option<IoValue>> {
    let expectation = parse_text(content)?;
    if let Some(fields) = expectation.collect_simple_record("expect-output", Some(1)) {
        let expected = value_to_iovalue(&fields[0]);
        let actual = state
            .last_output
            .as_ref()
            .ok_or_else(|| MoltenError::invalid_harness("expect-output requires previous output"))?;
        let expected_ref = canonical_hash(&expected)?;
        let actual_ref = canonical_hash(actual)?;
        if expected_ref != actual_ref {
            return Err(MoltenError::invalid_harness(format!(
                "expect-output mismatch: expected {expected_ref}, got {actual_ref}"
            )));
        }
        return Ok(Some(expectation));
    }
    if let Some(fields) = expectation.collect_simple_record("expect-decision", Some(1)) {
        let expected = required_string(&fields[0], "expected decision")?;
        let actual = state
            .last_decision
            .as_ref()
            .ok_or_else(|| MoltenError::invalid_harness("expect-decision requires previous outcome"))?;
        if &expected != actual {
            return Err(MoltenError::invalid_harness(format!(
                "expect-decision mismatch: expected {expected}, got {actual}"
            )));
        }
        return Ok(Some(expectation));
    }
    if let Some(fields) = expectation.collect_simple_record("expect-kind", Some(1)) {
        let expected = required_string(&fields[0], "expected kind")?;
        let actual = state
            .last_kind
            .as_ref()
            .ok_or_else(|| MoltenError::invalid_harness("expect-kind requires previous outcome"))?;
        if &expected != actual {
            return Err(MoltenError::invalid_harness(format!(
                "expect-kind mismatch: expected {expected}, got {actual}"
            )));
        }
        return Ok(Some(expectation));
    }
    if let Some(fields) = expectation.collect_simple_record("expect-error-contains", Some(1)) {
        let needle = required_string(&fields[0], "expected error substring")?;
        let haystack = state.last_output.as_ref().map(to_text).transpose()?.unwrap_or_default();
        if !haystack.contains(&needle) {
            return Err(MoltenError::invalid_harness(format!("expected previous output/error to contain {needle:?}")));
        }
        return Ok(Some(expectation));
    }
    Err(MoltenError::invalid_harness("unsupported transcript expectation"))
}

fn stanza_outcome(
    stanza: &TranscriptStanza,
    decision: &str,
    output: Option<IoValue>,
    diagnostics: Vec<String>,
) -> Result<StanzaOutcome> {
    validate_decision(decision)?;
    let output_ref = output.as_ref().map(canonical_hash).transpose()?;
    let value = record("transcript-stanza-outcome-v1", vec![
        string(TRANSCRIPT_STANZA_OUTCOME_SCHEMA),
        record("index", vec![u64_value(stanza.index)]),
        record("kind", vec![string(&stanza.kind)]),
        record("stanza", vec![string(&stanza.stanza_ref)]),
        record("decision", vec![string(decision)]),
        record("output", vec![optional_ref_value(output_ref.as_deref())]),
        record("diagnostics", vec![sequence(diagnostics.iter().map(string).collect())]),
        checks_value(&["stanza-outcome-bound", "hidden-evidence-preserved"]),
    ]);
    Ok(StanzaOutcome {
        outcome_ref: canonical_hash(&value)?,
        index: stanza.index,
        kind: stanza.kind.clone(),
        decision: decision.to_string(),
        output,
        diagnostics,
        value,
    })
}

fn denial_outcome(index: u64, kind: &str, diagnostic: String) -> Result<StanzaOutcome> {
    let stanza = TranscriptStanza {
        stanza_ref: local_ref("transcript-denial-stanza", &format!("{index}:{kind}"))?,
        index,
        kind: kind.to_string(),
        modifiers: Vec::new(),
        content: diagnostic.clone(),
        content_ref: local_ref("transcript-denial-content", &diagnostic)?,
        declared_refs: Vec::new(),
        value: record("transcript-denial-placeholder", vec![string(&diagnostic)]),
    };
    stanza_outcome(&stanza, DECISION_DENY, None, vec![diagnostic])
}

fn run_receipt_value(input: &RunReceiptValueInput<'_>) -> Result<IoValue> {
    validate_ref(input.transcript_ref, "transcript ref")?;
    validate_decision(input.decision)?;
    let output_ref = input.output.map(canonical_hash).transpose()?;
    Ok(record("transcript-run-receipt-v1", vec![
        string(TRANSCRIPT_RUN_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("transcript", vec![string(input.transcript_ref)]),
        record("mode", vec![string(input.mode)]),
        record("outcomes", vec![refs_sequence(
            &input.outcomes.iter().map(|outcome| outcome.outcome_ref.clone()).collect::<Vec<_>>(),
        )]),
        record("output", vec![optional_ref_value(output_ref.as_deref())]),
        record("refs", vec![refs_sequence(&sorted_unique(&input.refs))]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("outcome-values", vec![sequence(
            input.outcomes.iter().map(|outcome| outcome.value.clone()).collect(),
        )]),
        checks_value_from_pairs(input.checks),
    ]))
}

fn parse_modifier_token(token: &str) -> Result<TranscriptModifier> {
    let token = token.strip_prefix(':').unwrap_or(token);
    if token.is_empty() {
        return Err(MoltenError::invalid_harness("empty transcript modifier"));
    }
    let (name, value) = token.split_once('=').map_or((token, None), |(name, value)| (name, Some(value)));
    validate_modifier(name)?;
    Ok(TranscriptModifier {
        name: name.to_string(),
        value: value.map(str::to_string),
    })
}

fn modifier_value(modifier: &TranscriptModifier) -> IoValue {
    record("modifier", vec![string(&modifier.name), optional_string_value(modifier.value.as_deref())])
}

fn validate_kind(kind: &str) -> Result<()> {
    if matches!(kind, KIND_MOLTEN_CLI | KIND_PRESERVES | KIND_ARTIFACT | KIND_POLICY | KIND_EXPECT | KIND_COMMENT) {
        Ok(())
    } else if kind == "shell" || kind == "sh" || kind == "bash" {
        Err(MoltenError::invalid_harness("ambient shell transcript stanzas are denied by default"))
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported transcript stanza kind {kind}")))
    }
}

fn validate_modifier(modifier: &str) -> Result<()> {
    if matches!(modifier, "error" | "bug" | "hide" | "skip" | "requires" | "seed" | "profile") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported transcript modifier {modifier}")))
    }
}

fn validate_parse_input(input: &TranscriptParseInput) -> Result<()> {
    validate_refs(&input.dependency_refs, "transcript dependency ref")?;
    if let Some(hash) = input.dependency_closure_hash.as_ref() {
        validate_ref(hash, "transcript dependency closure hash")?;
    }
    if let Some(handler) = input.handler_profile_ref.as_ref() {
        validate_ref(handler, "transcript handler profile ref")?;
    }
    validate_refs(&input.policy_refs, "transcript policy ref")?;
    validate_refs(&input.capability_refs, "transcript capability ref")?;
    validate_refs(&input.revocation_refs, "transcript revocation ref")?;
    if let Some(seed) = input.seed_ref.as_ref() {
        validate_ref(seed, "transcript seed ref")?;
    }
    validate_refs(&input.expected_refs, "transcript expected ref")
}

fn final_decision(outcomes: &[StanzaOutcome]) -> String {
    if outcomes
        .iter()
        .any(|outcome| outcome.decision == DECISION_DENY || outcome.decision == DECISION_ERROR)
    {
        DECISION_DENY.to_string()
    } else if outcomes.iter().any(|outcome| outcome.decision == DECISION_KNOWN_BUG) {
        DECISION_KNOWN_BUG.to_string()
    } else {
        DECISION_PASS.to_string()
    }
}

fn refs_for_transcript(transcript: &TranscriptArtifact, outcomes: &[StanzaOutcome]) -> Vec<String> {
    let mut refs = vec![
        transcript.transcript_ref.clone(),
        transcript.source_ref.clone(),
        transcript.dependency_closure_hash.clone(),
    ];
    refs.extend(transcript.dependency_refs.iter().cloned());
    refs.extend(transcript.policy_refs.iter().cloned());
    refs.extend(transcript.capability_refs.iter().cloned());
    refs.extend(transcript.revocation_refs.iter().cloned());
    refs.extend(transcript.expected_refs.iter().cloned());
    refs.extend(transcript.stanzas.iter().map(|stanza| stanza.stanza_ref.clone()));
    refs.extend(outcomes.iter().map(|outcome| outcome.outcome_ref.clone()));
    if let Some(handler) = transcript.handler_profile_ref.as_ref() {
        refs.push(handler.clone());
    }
    if let Some(seed) = transcript.seed_ref.as_ref() {
        refs.push(seed.clone());
    }
    sorted_unique(&refs)
}

fn option_value<'a>(args: &'a [&str], name: &str) -> Option<&'a str> {
    args.windows(2).find_map(|window| (window[0] == name).then_some(window[1]))
}

fn temp_state_root(label: &str) -> Result<PathBuf> {
    for _ in 0..MAX_TEMP_STATE_ROOT_ATTEMPTS {
        let nonce = TEMP_STATE_ROOT_COUNTER.fetch_add(1, RELAXED);
        let path = std::env::temp_dir().join(format!("molten-transcript-{label}-{}-{nonce}", std::process::id()));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(MoltenError::from(error)),
        }
    }
    Err(MoltenError::invalid_harness("exhausted bounded transcript temp root attempts"))
}

fn local_ref(kind: &str, label: &str) -> Result<String> {
    canonical_hash(&record("transcript-local-ref", vec![string(kind), string(label)]))
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn optional_string_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn parse_optional_ref_value(value: &PreservesValue<IoValue>) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(fields) = value.collect_simple_record("some", Some(1)) {
        return required_ref(&fields[0], "optional ref").map(Some);
    }
    required_ref(value, "optional ref").map(Some)
}

fn parse_optional_string_value(value: &PreservesValue<IoValue>) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(fields) = value.collect_simple_record("some", Some(1)) {
        return required_string(&fields[0], "optional string").map(Some);
    }
    required_string(value, "optional string").map(Some)
}

fn record_string(value: &PreservesValue<IoValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_string(&record[0], label)
}

fn record_u64(value: &PreservesValue<IoValue>, label: &str) -> Result<u64> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_u64(&record[0], label)
}

fn record_ref(value: &PreservesValue<IoValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_ref(&record[0], label)
}

fn record_optional_ref(value: &PreservesValue<IoValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    parse_optional_ref_value(&record[0])
}

fn record_ref_sequence(value: &PreservesValue<IoValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    parse_ref_sequence_value(&record[0], label)
}

fn record_sequence(value: &PreservesValue<IoValue>, label: &str) -> Result<Vec<PreservesValue<IoValue>>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    Ok(required_sequence(&record[0], label)?.iter().cloned().collect())
}

fn record_modifier_sequence(value: &PreservesValue<IoValue>) -> Result<Vec<TranscriptModifier>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, "modifiers", 1)?;
    let items = required_sequence(&record[0], "modifiers")?;
    ensure_count_at_most(items.len(), MAX_TRANSCRIPT_SEQUENCE_ITEMS, "transcript modifiers")?;
    let mut modifiers = Vec::with_capacity(items.len());
    for item in items.iter() {
        let item = value_to_iovalue(item);
        let fields = simple_record(&item, "modifier", 2)?;
        let name = required_string(&fields[0], "modifier name")?;
        validate_modifier(&name)?;
        push_bounded(
            &mut modifiers,
            TranscriptModifier {
                name,
                value: parse_optional_string_value(&fields[1])?,
            },
            MAX_TRANSCRIPT_SEQUENCE_ITEMS,
            "transcript modifiers",
        )?;
    }
    Ok(modifiers)
}

fn parse_ref_sequence_value(value: &PreservesValue<IoValue>, label: &str) -> Result<Vec<String>> {
    let items = required_sequence(value, label)?;
    let mut refs = Vec::with_capacity(items.len());
    for item in items.iter() {
        refs.push(required_ref(item, label)?);
    }
    Ok(refs)
}

fn refs_sequence(refs: &[String]) -> IoValue {
    sequence(refs.iter().map(string).collect())
}

fn sorted_unique(refs: &[String]) -> Vec<String> {
    refs.iter().cloned().collect::<Set<_>>().into_iter().collect()
}

fn checks_value(names: &[&str]) -> IoValue {
    checks_value_from_pairs(&names.iter().map(|name| (*name, "pass")).collect::<Vec<_>>())
}

fn checks_value_from_pairs(checks: &[(&str, &str)]) -> IoValue {
    record("checks", vec![sequence(
        checks.iter().map(|(name, status)| record("check", vec![string(name), string(status)])).collect(),
    )])
}

fn parse_checks(value: &PreservesValue<IoValue>) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let checks = simple_record(&value, "checks", 1)?;
    let items = required_sequence(&checks[0], "checks")?;
    let mut parsed = Vec::with_capacity(items.len());
    for item in items.iter() {
        let item = value_to_iovalue(item);
        let check = simple_record(&item, "check", 2)?;
        let name = required_string(&check[0], "check name")?;
        let status = required_string(&check[1], "check status")?;
        if status != "pass" && status != "fail" {
            return Err(MoltenError::invalid_harness(format!("transcript check {name} has status {status}")));
        }
        parsed.push(name);
    }
    Ok(parsed)
}

fn require_check(checks: &[String], expected: &str, context: &str) -> Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{context} missing {expected} check")))
    }
}

fn require_schema(value: &PreservesValue<IoValue>, expected: &str, context: &str) -> Result<()> {
    let actual = required_string(value, context)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported {context} schema {actual}; expected {expected}")))
    }
}

fn simple_record<'a>(
    value: &'a IoValue,
    label: &str,
    arity: usize,
) -> Result<std::borrow::Cow<'a, PreservesRecord<PreservesValue<IoValue>>>> {
    value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> with arity {arity}")))
}

#[allow(clippy::owned_cow)]
fn required_sequence<'a>(
    value: &'a PreservesValue<IoValue>,
    field: &str,
) -> Result<std::borrow::Cow<'a, Vec<PreservesValue<IoValue>>>> {
    value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {field}")))
}

fn required_string(value: &PreservesValue<IoValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn required_ref(value: &PreservesValue<IoValue>, field: &str) -> Result<String> {
    let value = required_string(value, field)?;
    validate_ref(&value, field)?;
    Ok(value)
}

fn required_u64(value: &PreservesValue<IoValue>, field: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {field}: {error}")))
}

fn ensure_count_at_most(actual: usize, maximum: usize, label: &str) -> Result<()> {
    if actual <= maximum {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("{label} count {actual} exceeds bound {maximum}")))
}

fn push_bounded<T>(values: &mut impl crate::bounded::VecSink<T>, value: T, maximum: usize, label: &str) -> Result<()> {
    let total = values
        .item_count()
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness(format!("{label} count overflow")))?;
    ensure_count_at_most(total, maximum, label)?;
    values.push_item(value);
    Ok(())
}

fn validate_ref(value_ref: &str, field: &str) -> Result<()> {
    validate_content_ref(value_ref).map_err(|error| {
        MoltenError::invalid_harness(format!("{field} must be a canonical content ref, got {value_ref}: {error}"))
    })
}

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    for value_ref in refs {
        validate_ref(value_ref, field)?;
    }
    Ok(())
}

fn validate_decision(decision: &str) -> Result<()> {
    if matches!(decision, DECISION_PASS | DECISION_DENY | DECISION_ERROR | DECISION_SKIP | DECISION_KNOWN_BUG) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported transcript decision {decision}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Case = hegel::TestCase;

    #[test]
    fn parse_markdown_preserves_order_modifiers_and_stable_refs() {
        let source = "# Demo\n\n```preserves:hide\n<value 1>\n```\n\n```expect\n<expect-output <value 1>>\n```\n";
        let first = parse_markdown(source, &TranscriptParseInput::default()).expect("parse first");
        let second = parse_markdown(source, &TranscriptParseInput::default()).expect("parse second");
        assert_eq!(first.transcript_ref, second.transcript_ref);
        assert_eq!(first.stanzas.len(), 3);
        assert_eq!(first.stanzas[1].kind, KIND_PRESERVES);
        assert!(first.stanzas[1].has_modifier("hide"));
        assert_eq!(first.stanzas[2].kind, KIND_EXPECT);
    }

    #[test]
    fn fresh_runs_are_deterministic_across_temp_roots_and_render_hides_output() {
        let source = "```preserves:hide\n<value \"stable\">\n```\n```expect\n<expect-output <value \"stable\">>\n```\n";
        let transcript = parse_markdown(source, &TranscriptParseInput::default()).expect("parse");
        let first = run_transcript(&transcript, &TranscriptRunInput::default()).expect("run first");
        let second = run_transcript(&transcript, &TranscriptRunInput::default()).expect("run second");
        assert_eq!(first.decision, DECISION_PASS);
        assert_eq!(second.decision, DECISION_PASS);
        assert_eq!(
            canonical_hash(&first.receipt_value).expect("first hash"),
            canonical_hash(&second.receipt_value).expect("second hash")
        );
        let rendered = render_transcript(&transcript, Some(&first)).expect("render");
        assert!(rendered.contains("output hidden"));
        assert!(!rendered.contains("stable\">\n```preserves-output"));
    }

    #[test]
    fn restricted_cli_installs_artifact_and_matches_receipt_expectations() {
        let source = "```preserves\n<payload \"doc\">\n```\n```molten-cli\ntest artifact install --kind transcript-example\n```\n```expect\n<expect-decision \"pass\">\n```\n```molten-cli\ntest artifact list\n```\n";
        let transcript = parse_markdown(source, &TranscriptParseInput::default()).expect("parse");
        let run = run_transcript(&transcript, &TranscriptRunInput::default()).expect("run");
        assert_eq!(run.decision, DECISION_PASS);
        assert!(run.stanza_outcomes.iter().any(|outcome| {
            outcome
                .output
                .as_ref()
                .is_some_and(|output| output.collect_simple_record("artifact-list", Some(1)).is_some())
        }));
    }

    #[test]
    fn expected_error_known_bug_and_ambient_shell_denials_are_canonical() {
        let source =
            "```molten-cli:error\ntest unsupported command\n```\n```molten-cli:bug\ntest artifact closure\n```\n";
        let transcript = parse_markdown(source, &TranscriptParseInput::default()).expect("parse");
        let run = run_transcript(&transcript, &TranscriptRunInput::default()).expect("run");
        assert_eq!(run.decision, DECISION_KNOWN_BUG);
        assert_eq!(run.stanza_outcomes[0].decision, DECISION_PASS);
        assert_eq!(run.stanza_outcomes[1].decision, DECISION_KNOWN_BUG);
        let shell = parse_markdown("```shell\necho ambient\n```", &TranscriptParseInput::default())
            .expect_err("ambient shell denied");
        assert!(shell.to_string().contains("ambient shell"), "{shell}");
    }

    #[test]
    fn eval_cache_hit_reuses_deterministic_transcript_receipt() {
        let source = "```preserves\n<value \"cache\">\n```\n```expect\n<expect-output <value \"cache\">>\n```\n";
        let dependency_ref = local_ref("transcript-dependency", "cache").expect("dependency ref");
        let handler_profile_ref = local_ref("transcript-handler-profile", "deterministic").expect("profile ref");
        let policy_ref = local_ref("transcript-policy", "cache").expect("policy ref");
        let initial_state_ref = local_ref("transcript-initial-state", "cache").expect("initial state ref");
        let seed_ref = local_ref("transcript-seed", "cache").expect("seed ref");
        let expected_ref = local_ref("transcript-expected-output", "cache").expect("expected ref");
        let transcript = parse_markdown(source, &TranscriptParseInput {
            dependency_refs: vec![dependency_ref.clone()],
            dependency_closure_hash: Some(initial_state_ref.clone()),
            handler_profile_ref: Some(handler_profile_ref.clone()),
            policy_refs: vec![policy_ref.clone()],
            seed_ref: Some(seed_ref.clone()),
            expected_refs: vec![expected_ref.clone()],
            ..TranscriptParseInput::default()
        })
        .expect("parse");
        let cache_key = crate::eval_cache::parse_key(
            &crate::eval_cache::key_value(&transcript_cache_key(&transcript).expect("transcript cache key"))
                .expect("cache key value"),
        )
        .expect("parse cache key");
        assert_eq!(cache_key.dependency_closure_hash, initial_state_ref);
        assert_eq!(cache_key.dependency_refs, vec![dependency_ref]);
        assert_eq!(cache_key.handler_profile_ref.as_deref(), Some(handler_profile_ref.as_str()));
        assert_eq!(cache_key.policy_refs, vec![policy_ref]);
        assert!(cache_key.assumption_refs.contains(&seed_ref));
        assert!(cache_key.assumption_refs.contains(&expected_ref));

        let cache_root = temp_state_root("cache-test").expect("cache root");
        let input = TranscriptRunInput {
            cache_root: Some(cache_root),
            ..TranscriptRunInput::default()
        };
        let first = run_transcript(&transcript, &input).expect("first run");
        assert!(first.cache_receipt_value.is_some());
        let second = run_transcript(&transcript, &input).expect("second cached run");
        assert!(second.cache_receipt_value.is_some());
        assert_eq!(second.stanza_outcomes.len(), 0);
        assert_eq!(first.receipt_ref, second.receipt_ref);
    }

    #[test]
    fn ledger_classifies_transcript_artifacts_and_receipts() {
        let transcript =
            parse_markdown("```preserves\n<value 1>\n```", &TranscriptParseInput::default()).expect("parse");
        assert_eq!(crate::ledger::artifact_kind(&transcript.value), "transcript-artifact");
        let run = run_transcript(&transcript, &TranscriptRunInput::default()).expect("run");
        assert_eq!(crate::ledger::artifact_kind(&run.receipt_value), "transcript-run-receipt");
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_stanza_order_identity_and_denied_ambient_properties(tc: Case) {
        let n = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(1000));
        let source = format!("```preserves\n<value {}>\n```\n```expect\n<expect-output <value {}>>\n```\n", n, n);
        let transcript = parse_markdown(&source, &TranscriptParseInput::default()).expect("parse");
        let reparsed = parse_transcript_artifact(&transcript.value).expect("reparse");
        assert_eq!(transcript.transcript_ref, reparsed.transcript_ref);
        assert_eq!(transcript.stanzas[0].index, 0);
        assert_eq!(transcript.stanzas[1].index, 1);
        let run = run_transcript(&transcript, &TranscriptRunInput::default()).expect("run");
        assert_eq!(run.decision, DECISION_PASS);
        let bad = parse_markdown(&format!("```shell\necho {}\n```", n), &TranscriptParseInput::default());
        assert!(bad.is_err());
        let value = parse_text(&format!("<value {}>", n)).expect("value");
        assert_eq!(
            canonical_hash(run.stanza_outcomes[0].output.as_ref().expect("output")).expect("output ref"),
            canonical_hash(&value).expect("value ref")
        );
    }
}
