use std::io;
use std::path;

use super::admission;
use super::fixture;
use super::profile;

fn scenario(value: &fixture::Scenario) -> preserves::IOValue {
    let (tag, fields) = match value {
        fixture::Scenario::Standard => ("standard", Vec::new()),
        fixture::Scenario::StaleGeneration => ("stale-generation", Vec::new()),
        fixture::Scenario::HostPause => ("host-pause", Vec::new()),
        fixture::Scenario::HostTrap { reason } => ("host-trap", vec![crate::codec::string(reason)]),
        fixture::Scenario::HostUncertain { reason } => ("host-uncertain", vec![crate::codec::string(reason)]),
        fixture::Scenario::Cancel { after_segments, reason } => ("cancel", vec![
            crate::codec::u64_value(*after_segments as u64),
            crate::codec::string(reason),
        ]),
    };
    crate::codec::record(tag, fields)
}

fn outcome(value: &fixture::Outcome) -> preserves::IOValue {
    match value {
        fixture::Outcome::Finished => crate::codec::record("finished", Vec::new()),
        fixture::Outcome::ScheduleExhausted => crate::codec::record("schedule-exhausted", Vec::new()),
        fixture::Outcome::Failed(error) => crate::codec::record("failed", vec![error.value()]),
        fixture::Outcome::ProofFailed(error) => crate::codec::record("proof-failed", vec![error.value()]),
    }
}

fn blocker(value: &admission::Blocker) -> preserves::IOValue {
    let (tag, detail) = match value {
        admission::Blocker::ProfileAbsent => ("profile-absent", None),
        admission::Blocker::EngineMismatch => ("engine-mismatch", None),
        admission::Blocker::Profile(reason) => ("profile", Some(reason)),
        admission::Blocker::ArtifactMismatch => ("artifact-mismatch", None),
        admission::Blocker::Module(reason) => ("module", Some(reason)),
        admission::Blocker::StaleAuthority => ("stale-authority", None),
        admission::Blocker::StaleResources => ("stale-resources", None),
        admission::Blocker::HostcallDenied(name) => ("hostcall-denied", Some(name)),
        admission::Blocker::MissingEvidence => ("missing-evidence", None),
        admission::Blocker::EvidenceArtifactMismatch => ("evidence-artifact-mismatch", None),
    };
    crate::codec::record(tag, detail.map(|value| vec![crate::codec::string(value)]).unwrap_or_default())
}

/// A diagnostic reference never upgrades consumer-owned observations to release evidence.
pub fn value(report: &fixture::Report, diagnostic: Option<&str>) -> preserves::IOValue {
    let observation = report.observation.as_ref().map(|value| vec![value.value()]).unwrap_or_default();
    let events = report.events.iter().map(|event| event.value()).collect();
    let blockers = report.blockers.iter().map(crate::codec::string).collect();
    let diagnostic = diagnostic.map(|value| vec![crate::codec::string(value)]).unwrap_or_default();
    crate::codec::record("molten-spacewasm-fixture-status-v1", vec![
        crate::codec::string(profile::ID),
        crate::codec::string(profile::REVISION),
        crate::codec::string(&report.name),
        scenario(&report.scenario),
        crate::codec::string(&report.binding.artifact),
        crate::codec::string(&report.binding.profile),
        crate::codec::string(&report.evidence),
        crate::codec::string(&report.binding.actor),
        crate::codec::u64_value(report.binding.generation),
        crate::codec::string(&report.binding.input),
        crate::codec::string(&report.binding.authority),
        crate::codec::string(&report.binding.resource),
        crate::codec::string(&report.binding.effects),
        crate::codec::bool_value(report.passed),
        outcome(&report.outcome),
        crate::codec::sequence(observation),
        crate::codec::sequence(events),
        crate::codec::u64_value(report.host_dispatches as u64),
        crate::codec::u64_value(report.host_pauses as u64),
        crate::codec::bool_value(report.cleanup),
        crate::codec::bool_value(report.stale_generation_observed),
        crate::codec::sequence(blockers),
        crate::codec::sequence(diagnostic),
        crate::codec::bool_value(false),
        crate::codec::string("experimental-not-release-evidence"),
    ])
}

pub fn identity(report: &fixture::Report, diagnostic: Option<&str>) -> Result<String, String> {
    crate::codec::canonical_hash(&value(report, diagnostic)).map_err(|error| error.to_string())
}

pub fn denial_value(report: &fixture::DeniedReport, diagnostic: Option<&str>) -> preserves::IOValue {
    let blockers = report.blockers.iter().map(blocker).collect();
    let diagnostic = diagnostic.map(|value| vec![crate::codec::string(value)]).unwrap_or_default();
    crate::codec::record("molten-spacewasm-fixture-admission-status-v1", vec![
        crate::codec::string(profile::ID),
        crate::codec::string(profile::REVISION),
        crate::codec::string(&report.name),
        crate::codec::string(&report.artifact),
        crate::codec::string(&report.profile),
        crate::codec::string(&report.evidence),
        crate::codec::string(&report.actor),
        crate::codec::u64_value(report.generation),
        crate::codec::string(&report.authority),
        crate::codec::string(&report.resource),
        crate::codec::bool_value(report.passed),
        crate::codec::sequence(blockers),
        crate::codec::sequence(diagnostic),
        crate::codec::bool_value(false),
        crate::codec::string("experimental-not-release-evidence"),
    ])
}

pub fn replay_value(replay: &fixture::Replay, diagnostic: Option<&str>) -> preserves::IOValue {
    crate::codec::record("molten-spacewasm-fixture-replay-status-v1", vec![
        value(&replay.segmented, diagnostic),
        value(&replay.uninterrupted, diagnostic),
        crate::codec::string(&replay.receipt),
        crate::codec::bool_value(false),
        crate::codec::string("experimental-not-release-evidence"),
    ])
}

#[derive(Clone, Copy)]
pub enum Kind {
    Execution,
    AdmissionDenial,
    Replay,
}

pub struct Expected<'a> {
    pub kind: Kind,
    pub receipt: &'a str,
    /// One profile for execution/denial; segmented then uninterrupted for replay.
    pub profiles: &'a [&'a str],
}

pub struct Readback {
    identity: String,
    fixture: String,
    passed: bool,
    value: preserves::IOValue,
}

impl Readback {
    pub fn value(&self) -> &preserves::IOValue {
        &self.value
    }
    pub fn identity(&self) -> &str {
        &self.identity
    }
    pub fn render(&self) -> String {
        format!(
            "SpaceWasm fixture: {}\nStatus: {}\nReceipt: {}\nProduction evidence: false\nexperimental-not-release-evidence",
            self.fixture,
            if self.passed { "passed" } else { "blocked" },
            self.identity
        )
    }
}

/// Bounded, capability-relative canonical readback; identity/profile expectations
/// come from the operator's receipt reference, never from the submitted file.
pub fn readback(root: &cap_std::fs::Dir, relative: &path::Path, expected: Expected<'_>) -> Result<Readback, String> {
    if relative.as_os_str().is_empty() || relative.components().any(|part| !matches!(part, path::Component::Normal(_)))
    {
        return Err("status path must be capability-relative without traversal".into());
    }
    const MAX_BYTES: u64 = 1_048_576;
    let file = root.open(relative).map_err(|error| error.to_string())?;
    if file.metadata().map_err(|error| error.to_string())?.len() > MAX_BYTES {
        return Err("status receipt exceeds readback bound".into());
    }
    let mut bounded = io::Read::take(file, MAX_BYTES + 1);
    let mut bytes = Vec::new();
    io::Read::read_to_end(&mut bounded, &mut bytes).map_err(|error| error.to_string())?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("status receipt exceeds readback bound".into());
    }
    let value = crate::codec::parse_canonical_bytes(&bytes).map_err(|error| error.to_string())?;
    let identity = crate::codec::canonical_hash(&value).map_err(|error| error.to_string())?;
    if identity != expected.receipt {
        return Err("status receipt identity mismatch".into());
    }
    let (fixture, passed) = validate(&value, expected.kind, expected.profiles)?;
    Ok(Readback {
        identity,
        fixture,
        passed,
        value,
    })
}

fn validate(value: &preserves::IOValue, kind: Kind, profiles: &[&str]) -> Result<(String, bool), String> {
    if matches!(kind, Kind::Replay) {
        if profiles.len() != 2 {
            return Err("replay requires both expected profiles".into());
        }
        let fields = crate::codec::simple_record_fields(value, "molten-spacewasm-fixture-replay-status-v1", 5)
            .map_err(|error| error.to_string())?;
        if fields[3].as_boolean() != Some(false)
            || fields[4].as_string().as_deref() != Some("experimental-not-release-evidence")
        {
            return Err("production evidence overclaim".into());
        }
        crate::codec::required_content_ref_string(&fields[2], "replay receipt").map_err(|error| error.to_string())?;
        let left = validate(&crate::codec::value_to_iovalue(&fields[0]), Kind::Execution, &profiles[..1])?;
        let right = validate(&crate::codec::value_to_iovalue(&fields[1]), Kind::Execution, &profiles[1..])?;
        if left.0 != right.0 {
            return Err("replay fixture identities differ".into());
        }
        return Ok((left.0, left.1 && right.1));
    }
    if profiles.len() != 1 {
        return Err("one expected profile required".into());
    }
    let (label, arity, profile_index, passed_index, production_index) = match kind {
        Kind::Execution => ("molten-spacewasm-fixture-status-v1", 25, 5, 13, 23),
        Kind::AdmissionDenial => ("molten-spacewasm-fixture-admission-status-v1", 15, 4, 10, 13),
        Kind::Replay => return Err("invalid nested replay".into()),
    };
    let fields = crate::codec::simple_record_fields(value, label, arity).map_err(|error| error.to_string())?;
    if fields[0].as_string().as_deref() != Some(profile::ID)
        || fields[1].as_string().as_deref() != Some(profile::REVISION)
        || fields[profile_index].as_string().as_deref() != Some(profiles[0])
    {
        return Err("status profile or source cohort mismatch".into());
    }
    if fields[production_index].as_boolean() != Some(false)
        || fields[production_index + 1].as_string().as_deref() != Some("experimental-not-release-evidence")
    {
        return Err("production evidence overclaim".into());
    }
    let name = crate::codec::required_string_field(&fields[2], "fixture").map_err(|error| error.to_string())?;
    if !fixture::NAMES.contains(&name.as_str()) {
        return Err("unknown fixture identity".into());
    }
    let passed = fields[passed_index].as_boolean().ok_or("status outcome must be boolean")?;
    Ok((name, passed))
}

#[cfg(test)]
mod tests {
    use super::super::continuation;
    fn sample() -> super::fixture::DeniedReport {
        super::fixture::DeniedReport {
            name: "molten-abi-memory-limit".into(),
            artifact: crate::codec::content_ref_from_bytes(b"artifact"),
            profile: crate::codec::content_ref_from_bytes(b"profile"),
            evidence: crate::codec::content_ref_from_bytes(b"evidence"),
            actor: "actor".into(),
            generation: 7,
            authority: crate::codec::content_ref_from_bytes(b"authority"),
            resource: crate::codec::content_ref_from_bytes(b"resource"),
            blockers: vec![super::admission::Blocker::Module(
                "explicit bounded MVP memory maximum required".into(),
            )],
            passed: true,
        }
    }

    fn root() -> (tempfile::TempDir, cap_std::fs::Dir) {
        let temporary = tempfile::tempdir().unwrap();
        let root = cap_std::fs::Dir::open_ambient_dir(temporary.path(), cap_std::ambient_authority()).unwrap();
        (temporary, root)
    }

    fn store(root: &cap_std::fs::Dir, value: &preserves::IOValue) -> String {
        root.write("receipt", crate::codec::canonical_bytes(value).unwrap()).unwrap();
        crate::codec::canonical_hash(value).unwrap()
    }

    #[test]
    fn readback_binds_identity_profile_record_kind_and_canonical_encoding() {
        let (_temporary, root) = root();
        let report = sample();
        let value = super::denial_value(&report, None);
        let identity = store(&root, &value);
        let profiles = [report.profile.as_str()];
        let expected = || super::Expected {
            kind: super::Kind::AdmissionDenial,
            receipt: &identity,
            profiles: &profiles,
        };
        let path = std::path::Path::new("receipt");
        drop(super::readback(&root, path, expected()).unwrap());
        assert!(
            super::readback(&root, path, super::Expected {
                receipt: "wrong",
                ..expected()
            })
            .is_err()
        );
        assert!(
            super::readback(&root, path, super::Expected {
                profiles: &["different"],
                ..expected()
            })
            .is_err()
        );
        assert!(
            super::readback(&root, path, super::Expected {
                kind: super::Kind::Execution,
                ..expected()
            })
            .is_err()
        );
        let mut bytes = crate::codec::canonical_bytes(&value).unwrap();
        bytes.push(0);
        root.write("receipt", bytes).unwrap();
        assert!(super::readback(&root, path, expected()).is_err());
    }

    #[test]
    fn matching_receipt_hash_cannot_authorize_production_overclaim() {
        let (_temporary, root) = root();
        let report = sample();
        let value = super::denial_value(&report, None);
        let mut fields: Vec<_> =
            crate::codec::simple_record_fields(&value, "molten-spacewasm-fixture-admission-status-v1", 15)
                .unwrap()
                .fields_iter()
                .map(crate::codec::value_to_iovalue)
                .collect();
        fields[13] = crate::codec::bool_value(true);
        let overclaim = crate::codec::record("molten-spacewasm-fixture-admission-status-v1", fields);
        let identity = store(&root, &overclaim);
        assert!(
            super::readback(&root, std::path::Path::new("receipt"), super::Expected {
                kind: super::Kind::AdmissionDenial,
                receipt: &identity,
                profiles: &[&report.profile],
            })
            .is_err()
        );
    }

    #[test]
    fn readback_refuses_ambient_paths_and_oversized_receipts() {
        let (temporary, root) = root();
        let report = sample();
        let profiles = [report.profile.as_str()];
        let identity = store(&root, &super::denial_value(&report, None));
        root.create_dir("inside").unwrap();
        let child = root.open_dir("inside").unwrap();
        let expected = || super::Expected {
            kind: super::Kind::AdmissionDenial,
            receipt: &identity,
            profiles: &profiles,
        };
        for path in [std::path::PathBuf::from("../receipt"), temporary.path().join("receipt")] {
            assert!(super::readback(&child, &path, expected()).is_err());
        }
        let diagnostic = "x".repeat(1_048_576);
        let oversized = super::denial_value(&report, Some(&diagnostic));
        let oversized_ref = store(&root, &oversized);
        assert!(
            super::readback(&root, std::path::Path::new("receipt"), super::Expected {
                receipt: &oversized_ref,
                ..expected()
            })
            .is_err()
        );
    }

    #[test]
    fn terminal_status_preserves_failure_phase_reason_and_cancel_schedule() {
        let failures = [
            continuation::Failure::Trap("same".into()),
            continuation::Failure::Uncertain("same".into()),
            continuation::Failure::Cancelled("same".into()),
            continuation::Failure::Denied("same".into()),
        ];
        let values: Vec<_> = failures
            .iter()
            .map(|failure| {
                let value = super::outcome(&super::fixture::Outcome::Failed(failure.clone()));
                let packed = crate::codec::canonical_bytes(&value).unwrap();
                assert_eq!(crate::codec::parse_canonical_bytes(&packed).unwrap(), value);
                let fields = crate::codec::simple_record_fields(&value, "failed", 1).unwrap();
                assert_eq!(crate::codec::value_to_iovalue(&fields[0]), failure.value());
                value
            })
            .collect();
        for (index, left) in values.iter().enumerate() {
            for right in &values[index + 1..] {
                assert_ne!(left, right);
            }
        }
        assert_ne!(
            super::outcome(&super::fixture::Outcome::ProofFailed(continuation::Failure::Proof("missing state".into()))),
            values[0]
        );
        let cancel = |after_segments| {
            super::scenario(&super::fixture::Scenario::Cancel {
                after_segments,
                reason: "operator request".into(),
            })
        };
        assert_ne!(cancel(0), cancel(1));
        let absent = super::super::receipt::Output::Absent.value();
        let rejected = super::super::receipt::Output::Rejected {
            bytes: vec![0xff],
            issue: "canonical-validation-failed".into(),
        }
        .value();
        let canonical = super::super::receipt::Output::Canonical(
            crate::codec::canonical_bytes(&crate::codec::bool_value(false)).unwrap(),
        )
        .value();
        assert_ne!(absent, rejected);
        assert_ne!(canonical, rejected);
        assert_ne!(canonical, absent);
    }
}
