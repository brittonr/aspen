use std::cell;
use std::rc;

use super::admission;
use super::allocation;
use super::continuation;
use super::engine;
use super::host;
use super::receipt;

pub const NAMES: [&str; 8] = [
    "molten-abi-roundtrip",
    "molten-abi-hostcall",
    "molten-abi-invalid-pointer",
    "molten-abi-invalid-output",
    "molten-abi-memory-limit",
    "molten-abi-stack-limit",
    "molten-abi-trap",
    "molten-abi-out-of-fuel",
];

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub enum Scenario {
    Standard,
    StaleGeneration,
    HostPause,
    HostTrap { reason: String },
    HostUncertain { reason: String },
    Cancel { after_segments: usize, reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExpectedOutput {
    Absent,
    Canonical(Vec<u8>),
    Rejected(Vec<u8>),
}

/// Expectations are supplied alongside exact verified fixture artifacts, not inferred
/// from a filename or manufactured by substituting a different module.
pub struct Expectation {
    pub scenario: Scenario,
    pub output: ExpectedOutput,
    pub hostcalls: Vec<receipt::Hostcall>,
}

pub struct Request<'a> {
    pub name: &'a str,
    pub bytes: &'a [u8],
    pub start: engine::Start,
    pub contexts: &'a [continuation::ResumeContext],
    pub expected: Expectation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Finished,
    Failed(continuation::Failure),
    ProofFailed(continuation::Failure),
    ScheduleExhausted,
}

pub struct Report {
    pub name: String,
    pub scenario: Scenario,
    pub binding: continuation::Binding,
    pub evidence: String,
    pub outcome: Outcome,
    pub observation: Option<receipt::Observation>,
    pub events: Vec<receipt::Event>,
    pub host_pauses: usize,
    pub host_dispatches: usize,
    pub stale_generation_observed: bool,
    pub cleanup: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

/// Every context resolves live admission, and every host interaction uses Start's
/// supplied HostPort. This diagnostic runner supplies neither authority nor replies.
pub fn run(mut request: Request<'_>) -> Result<Report, String> {
    if !NAMES.contains(&request.name) {
        return Err("unknown exact Mantle fixture family member".into());
    }
    if request.start.plan.fixture_name() != request.name {
        return Err("verified bundle member does not bind the requested fixture name".into());
    }
    if request.name == "molten-abi-memory-limit" {
        return Err("memory-limit fixture requires admission_denial with a one-page profile".into());
    }
    if matches!(request.name, "molten-abi-roundtrip" | "molten-abi-hostcall")
        && matches!(request.expected.scenario, Scenario::Standard | Scenario::HostPause)
        && !matches!(&request.expected.output, ExpectedOutput::Canonical(bytes) if bytes == &request.start.input)
    {
        return Err("Mantle positive fixtures require exact canonical input/output equality".into());
    }
    let dispatches = rc::Rc::new(cell::Cell::new(0));
    request.start.host = rc::Rc::new(ObservedHost {
        inner: request.start.host,
        dispatches: dispatches.clone(),
    });
    crate::codec::parse_canonical_bytes(&request.start.input).map_err(|error| error.to_string())?;
    let binding = request.start.binding.clone();
    let evidence = request.start.plan.evidence_ref().to_owned();
    let mut report = Report {
        name: request.name.into(),
        scenario: request.expected.scenario.clone(),
        binding,
        evidence,
        outcome: Outcome::ScheduleExhausted,
        observation: None,
        events: Vec::new(),
        host_pauses: 0,
        cleanup: false,
        passed: false,
        blockers: Vec::new(),
        host_dispatches: 0,
        stale_generation_observed: false,
    };
    match engine::Engine::start(request.bytes, request.start) {
        Err(error) => {
            report.outcome = if error.is_proof_error() {
                Outcome::ProofFailed(error)
            } else {
                Outcome::Failed(error)
            }
        }
        Ok(mut engine) => {
            for (segments, context) in request.contexts.iter().enumerate() {
                if request.expected.scenario == Scenario::StaleGeneration {
                    report.stale_generation_observed |= context.current.current().is_ok_and(|binding| {
                        binding.actor == report.binding.actor && binding.generation != report.binding.generation
                    });
                }
                let progress = match &request.expected.scenario {
                    Scenario::Cancel { after_segments, reason } if segments == *after_segments => {
                        engine.cancel(context, reason.clone())
                    }
                    _ => engine.resume(context),
                };
                match progress {
                    continuation::Progress::Yield { host_pause, .. } => {
                        report.host_pauses += usize::from(host_pause);
                    }
                    continuation::Progress::Terminal(observation) => {
                        report.outcome = match &observation.terminal {
                            receipt::Terminal::Finished => Outcome::Finished,
                            receipt::Terminal::Failed(error) => Outcome::Failed(error.clone()),
                        };
                        report.observation = Some(observation);
                        break;
                    }
                    continuation::Progress::ProofFailed(error) => {
                        report.outcome = Outcome::ProofFailed(error);
                        break;
                    }
                }
            }
            report.events = engine.cleanup();
        }
    }
    report.host_dispatches = dispatches.get();
    report.cleanup = allocation::Budget::enter(0).is_ok();
    assess(&mut report, &request.expected);
    Ok(report)
}

fn assess(report: &mut Report, expected: &Expectation) {
    if !matches_expected(report, expected) {
        report.blockers.push("terminal outcome does not match exact fixture expectation".into());
    }
    if let Some(observation) = &report.observation {
        let output_matches = match (&expected.output, &observation.output) {
            (ExpectedOutput::Absent, receipt::Output::Absent) => true,
            (ExpectedOutput::Canonical(left), receipt::Output::Canonical(right))
            | (ExpectedOutput::Rejected(left), receipt::Output::Rejected { bytes: right, .. }) => left == right,
            _ => false,
        };
        if !output_matches || observation.validate().is_err() {
            report.blockers.push("exact typed output or actual terminal state mismatched".into());
        }
        if observation.hostcalls != expected.hostcalls {
            report.blockers.push("hostcall transcript differs from exact expectation".into());
        }
        if let Err(error) = terminal_evidence(report, observation) {
            report.blockers.push(error);
        }
    } else {
        report.blockers.push("actual terminal observation absent".into());
    }
    if expected.scenario == Scenario::StaleGeneration && !report.stale_generation_observed {
        report.blockers.push("changed actor generation was not observed in supplied live admission".into());
    }
    if matches!(expected.scenario, Scenario::HostTrap { .. } | Scenario::HostUncertain { .. })
        && report.host_dispatches == 0
    {
        report.blockers.push("host failure expected but supplied HostPort was never dispatched".into());
    }
    if expected.scenario == Scenario::HostPause && report.host_pauses == 0 {
        report.blockers.push("host pause was not observed".into());
    }
    if !report.cleanup {
        report
            .blockers
            .push("NASA context remained active or retained allocated bytes after cleanup".into());
    }
    report.passed = report.blockers.is_empty();
}

fn matches_expected(report: &Report, expected: &Expectation) -> bool {
    match &expected.scenario {
        Scenario::StaleGeneration => {
            return matches!(&report.outcome,
            Outcome::Failed(continuation::Failure::Denied(reason)) if reason == "stale continuation");
        }
        Scenario::HostTrap { reason } => {
            return matches!(&report.outcome,
            Outcome::Failed(continuation::Failure::Trap(actual)) if actual == reason);
        }
        Scenario::HostUncertain { reason } => {
            return matches!(&report.outcome,
            Outcome::Failed(continuation::Failure::Uncertain(actual)) if actual == reason);
        }
        Scenario::Cancel { reason, .. } => {
            return matches!(&report.outcome,
            Outcome::Failed(continuation::Failure::Cancelled(actual)) if actual == reason);
        }
        Scenario::HostPause | Scenario::Standard => {}
    }
    match report.name.as_str() {
        "molten-abi-roundtrip" | "molten-abi-hostcall" => report.outcome == Outcome::Finished,
        "molten-abi-invalid-pointer" => matches!(report.outcome, Outcome::Failed(continuation::Failure::Memory)),
        "molten-abi-invalid-output" => matches!(report.outcome, Outcome::Failed(continuation::Failure::InvalidOutput)),
        "molten-abi-stack-limit" => matches!(&report.outcome, Outcome::Failed(continuation::Failure::Stack)),
        "molten-abi-trap" => matches!(&report.outcome,
            Outcome::Failed(continuation::Failure::Trap(reason)) if reason == "unreachable"),
        "molten-abi-out-of-fuel" => matches!(report.outcome, Outcome::Failed(continuation::Failure::TotalFuel)),
        _ => false,
    }
}

pub struct Replay {
    pub segmented: Report,
    pub uninterrupted: Report,
    pub receipt: String,
}

/// Both profiles must have been independently admitted against the same artifact,
/// input and authority. No plan or profile is mutated to manufacture a schedule.
pub fn replay(segmented: Request<'_>, uninterrupted: Request<'_>) -> Result<Replay, String> {
    compare_replay(run(segmented)?, run(uninterrupted)?)
}
pub fn compare_replay(segmented: Report, uninterrupted: Report) -> Result<Replay, String> {
    let (left, right) = (&segmented.binding, &uninterrupted.binding);
    if segmented.name != uninterrupted.name
        || left.actor != right.actor
        || left.generation != right.generation
        || left.artifact != right.artifact
        || left.input != right.input
        || left.authority != right.authority
        || left.resource != right.resource
        || left.effects != right.effects
    {
        return Err("replay binding differs beyond independently admitted profile".into());
    }
    if !segmented.passed || !uninterrupted.passed {
        return Err("fixture expectations failed before replay comparison".into());
    }
    validate_schedule(&segmented, &uninterrupted)?;
    let left = segmented.observation.as_ref().ok_or("segmented observation absent")?;
    let right = uninterrupted.observation.as_ref().ok_or("uninterrupted observation absent")?;
    terminal_evidence(&segmented, left)?;
    terminal_evidence(&uninterrupted, right)?;
    let receipt = receipt::replay(left, right)?;
    Ok(Replay {
        segmented,
        uninterrupted,
        receipt,
    })
}
fn terminal_evidence(report: &Report, observation: &receipt::Observation) -> Result<(), String> {
    let binding = report.binding.identity()?;
    let identity = observation.identity()?;
    if report
        .events
        .iter()
        .enumerate()
        .any(|(index, event)| event.binding != binding || event.sequence != index as u64 || event.resource.is_none())
    {
        return Err("execution events lack exact binding, sequence or actual resources".into());
    }
    let mut terminals = report.events.iter().filter(|event| event.phase == "terminal");
    if terminals.next().is_none_or(|event| event.detail != identity) || terminals.next().is_some() {
        return Err("terminal event does not bind actual observation".into());
    }
    if !report.events.windows(2).any(|pair| {
        pair[0].phase == "resource" && pair[0].detail == observation.resource_class && pair[1].phase == "terminal"
    }) {
        return Err("actual terminal resource event absent".into());
    }
    Ok(())
}

fn validate_schedule(segmented: &Report, uninterrupted: &Report) -> Result<(), String> {
    let yielded =
        |report: &Report| report.events.iter().any(|event| event.phase == "yield" && event.detail == "out-of-fuel");
    let boundary = |report: &Report| {
        !report.events.iter().any(|event| event.phase == "segment")
            && report
                .events
                .iter()
                .any(|event| event.phase == "boundary" && event.detail == "no-instruction-execution")
            && matches!(
                &report.outcome,
                Outcome::Failed(continuation::Failure::Denied(_) | continuation::Failure::Cancelled(_))
            )
    };
    if yielded(uninterrupted) || (!yielded(segmented) && !(boundary(segmented) && boundary(uninterrupted))) {
        return Err("actual executed segmentation or deterministic no-execution boundary not observed".into());
    }
    Ok(())
}

struct ObservedHost {
    inner: rc::Rc<dyn host::HostPort>,
    dispatches: rc::Rc<cell::Cell<usize>>,
}

impl host::HostPort for ObservedHost {
    fn dispatch(
        &self,
        binding: &continuation::Binding,
        name: &str,
        bytes: &[u8],
    ) -> Result<host::Reply, continuation::Failure> {
        self.dispatches.set(self.dispatches.get().saturating_add(1));
        self.inner.dispatch(binding, name, bytes)
    }
}

pub struct DeniedReport {
    pub name: String,
    pub artifact: String,
    pub profile: String,
    pub evidence: String,
    pub actor: String,
    pub generation: u64,
    pub authority: String,
    pub resource: String,
    pub blockers: Vec<admission::Blocker>,
    pub passed: bool,
}

/// The fixed two-page Mantle memory fixture must fail real admission at one page.
/// Missing evidence, stale authority or unrelated profile failures never pass.
pub fn admission_denial(name: &str, request: admission::Request<'_>) -> Result<DeniedReport, String> {
    if name != "molten-abi-memory-limit" {
        return Err("this fixture requires execution, not admission denial".into());
    }
    let evidence = request.evidence.ok_or("verified external evidence required")?;
    if evidence.fixture_name() != name {
        return Err("verified bundle member does not bind the memory-limit fixture".into());
    }
    let profile = request.profile.ok_or("explicit one-page profile required")?;
    if profile.limits.memory_pages != 1 {
        return Err("memory-limit diagnostic requires exactly one memory page".into());
    }
    let mut report = DeniedReport {
        name: name.into(),
        artifact: request.artifact_ref.into(),
        profile: profile.content_ref()?,
        evidence: evidence.evidence_ref().into(),
        actor: request.actor.into(),
        generation: request.generation,
        authority: request.authority_ref.into(),
        resource: request.resource_ref.into(),
        blockers: Vec::new(),
        passed: false,
    };
    match admission::admit(request) {
        Ok(_) => return Err("memory-limit artifact was unexpectedly admitted".into()),
        Err(blockers) => report.blockers = blockers,
    }
    report.passed = report.blockers
        == vec![admission::Blocker::Module(
            "explicit bounded MVP memory maximum required".into(),
        )];
    Ok(report)
}
