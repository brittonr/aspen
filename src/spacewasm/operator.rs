//! Explicit fixture commands with genuine isolated signing roots and actual engines.
use std::cell;
use std::mem;
use std::path;
use std::rc;

use super::admission;
use super::authority;
use super::continuation;
use super::engine;
use super::evidence;
use super::fixture;
use super::fixtureauthority;
use super::fixtureeffects;
use super::host;
use super::profile;
use super::receipt;
use super::replay;
use super::status;

mod preparation;
#[cfg(test)]
mod smoke;

pub struct Request<'a> {
    pub trust: &'a evidence::Trust,
    pub sources: evidence::Sources<'a>,
    pub profile: profile::Profile,
    pub name: &'a str,
    pub scenario: fixture::Scenario,
    pub output: &'a cap_std::fs::Dir,
}

pub fn run(request: Request<'_>) -> Result<preserves::IOValue, String> {
    let mut ready = preparation::prepare(&request)?;
    let value = if request.name == "molten-abi-memory-limit" {
        let report = ready.source.with_admission(|current| {
            fixture::admission_denial(request.name, admission_request(&request, &ready, current))
        })??;
        if !report.passed {
            return Err("memory fixture did not produce its exact expected admission denial".into());
        }
        status::denial_value(&report, None)
    } else {
        let report = fixture::run(execution(&request, &mut ready, None)?)?;
        if !report.passed {
            return Err("actual fixture did not satisfy its exact acceptance criteria".into());
        }
        status::value(&report, None)
    };
    publish(request.output, &value)?;
    Ok(value)
}

fn admission_request<'a>(
    request: &'a Request<'_>,
    ready: &'a preparation::Ready,
    current: admission::Authority<'a>,
) -> admission::Request<'a> {
    let binding = &ready.binding;
    admission::Request {
        engine: profile::ID,
        profile: Some(&request.profile),
        bytes: &ready.bytes,
        artifact_ref: &binding.artifact,
        actor: &binding.actor,
        generation: binding.generation,
        authority_ref: &binding.authority,
        resource_ref: &binding.resource,
        current,
        evidence: Some(&ready.verified),
    }
}

fn execution<'a>(
    request: &'a Request<'_>,
    ready: &'a mut preparation::Ready,
    host: Option<rc::Rc<dyn host::HostPort>>,
) -> Result<fixture::Request<'a>, String> {
    let plan = ready
        .source
        .with_admission(|current| admission::admit(admission_request(request, ready, current)))?
        .map_err(|_| "fixture module was denied before construction".to_owned())?;
    let host = host.unwrap_or_else(|| authorized(request, ready));
    let hostcalls = expected_calls(request, &ready.binding)?;
    let expected = fixture::Expectation {
        scenario: request.scenario.clone(),
        output: preparation::expected_output(request, ready)?,
        hostcalls,
    };
    let start = engine::Start {
        plan,
        binding: ready.binding.clone(),
        current: ready.source.clone(),
        host,
        input: mem::take(&mut ready.input),
        logical_turn: 1,
    };
    Ok(fixture::Request {
        name: request.name,
        bytes: &ready.bytes,
        start,
        contexts: &ready.contexts,
        expected,
    })
}

fn authorized(request: &Request<'_>, ready: &preparation::Ready) -> rc::Rc<dyn host::HostPort> {
    let effects = rc::Rc::new(fixtureeffects::Effects::new(request.scenario.clone()));
    rc::Rc::new(authority::AuthorizedHost {
        authority: ready.source.clone(),
        effects,
    })
}

fn expected_calls(request: &Request<'_>, binding: &continuation::Binding) -> Result<Vec<receipt::Hostcall>, String> {
    if request.name != "molten-abi-hostcall"
        || matches!(request.scenario, fixture::Scenario::StaleGeneration | fixture::Scenario::Cancel { .. })
    {
        return Ok(Vec::new());
    }
    let response = match &request.scenario {
        fixture::Scenario::HostTrap { reason } => {
            receipt::Response::Failed(continuation::Failure::Trap(reason.clone()))
        }
        fixture::Scenario::HostUncertain { reason } => {
            receipt::Response::Failed(continuation::Failure::Uncertain(reason.clone()))
        }
        _ => receipt::Response::Returned(crate::codec::content_ref_from_bytes(&fixtureeffects::Effects::response()?)),
    };
    Ok(vec![receipt::Hostcall {
        name: "assert".into(),
        request: binding.input.clone(),
        response,
        authority: binding.authority.clone(),
    }])
}

struct Stale {
    source: rc::Rc<fixtureauthority::Authority>,
    advanced: cell::Cell<bool>,
}
impl continuation::CurrentAdmission for Stale {
    fn current(&self) -> Result<continuation::Binding, String> {
        if !self.advanced.replace(true) {
            self.source.advance_generation();
        }
        continuation::CurrentAdmission::current(&*self.source)
    }
}

fn read(root: &cap_std::fs::Dir, relative: &path::Path, limit: u64) -> Result<Vec<u8>, String> {
    if relative.as_os_str().is_empty() || relative.components().any(|part| !matches!(part, path::Component::Normal(_)))
    {
        return Err("fixture source must be a capability-relative normal path".into());
    }
    let file = root.open(relative).map_err(|e| e.to_string())?;
    let mut reader = std::io::Read::take(file, limit + 1);
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut reader, &mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err("fixture source byte bound".into());
    }
    Ok(bytes)
}

fn publish(output: &cap_std::fs::Dir, value: &preserves::IOValue) -> Result<(), String> {
    let bytes = crate::codec::canonical_bytes(value).map_err(|e| e.to_string())?;
    if bytes.len() > 1_048_576 {
        return Err("fixture status byte bound".into());
    }
    let mut file = output
        .open_with("status.preserves", cap_std::fs::OpenOptions::new().write(true).create_new(true))
        .map_err(|e| e.to_string())?;
    std::io::Write::write_all(&mut file, &bytes).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Configuration {
    trust: evidence::Trust,
    profile: profile::Profile,
    name: String,
    scenario: fixture::Scenario,
    bundle: path::PathBuf,
    octet: path::PathBuf,
    module: path::PathBuf,
    probe: path::PathBuf,
    input: path::PathBuf,
    chaos_report: path::PathBuf,
}

impl Configuration {
    fn request<'a>(&'a self, root: &'a cap_std::fs::Dir, output: &'a cap_std::fs::Dir) -> Request<'a> {
        Request {
            trust: &self.trust,
            sources: evidence::Sources {
                root,
                bundle: &self.bundle,
                octet: &self.octet,
                module: &self.module,
                probe: &self.probe,
                input: &self.input,
                chaos_report: &self.chaos_report,
            },
            profile: self.profile.clone(),
            name: &self.name,
            scenario: self.scenario.clone(),
            output,
        }
    }
}

fn configuration(root: &cap_std::fs::Dir, relative: &path::Path) -> Result<Configuration, String> {
    serde_json::from_slice(&read(root, relative, 65536)?).map_err(|e| e.to_string())
}

/// The selected configuration is deployment authority input, not a submitted receipt.
pub fn run_configuration(
    root: &cap_std::fs::Dir,
    configuration_path: &path::Path,
    output: &cap_std::fs::Dir,
) -> Result<preserves::IOValue, String> {
    run(configuration(root, configuration_path)?.request(root, output))
}

/// Independently verifies and admits both real executions; never compares supplied receipts.
pub fn replay_configuration(
    root: &cap_std::fs::Dir,
    left_config: &path::Path,
    right_config: &path::Path,
    output: &cap_std::fs::Dir,
) -> Result<preserves::IOValue, String> {
    let left_config = configuration(root, left_config)?;
    let right_config = configuration(root, right_config)?;
    let left = left_config.request(root, output);
    let right = right_config.request(root, output);
    let mut left_ready = preparation::prepare(&left)?;
    let mut right_ready = preparation::prepare(&right)?;
    let recording = rc::Rc::new(replay::Recording::new(authorized(&left, &left_ready), 64));
    let segmented = fixture::run(execution(&left, &mut left_ready, Some(recording.clone()))?)?;
    let recorded = rc::Rc::new(replay::Replay::new(recording.entries()));
    let uninterrupted = fixture::run(execution(&right, &mut right_ready, Some(recorded.clone()))?)?;
    recorded.finish()?;
    let replay = fixture::compare_replay(segmented, uninterrupted)?;
    let value = status::replay_value(&replay, None);
    publish(output, &value)?;
    Ok(value)
}
