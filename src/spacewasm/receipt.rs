// r[impl molten.spacewasm_core.receipts]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hostcall {
    pub name: String,
    pub request: String,
    pub response: Response,
    pub authority: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Response {
    Returned(String),
    Failed(super::continuation::Failure),
}
impl Response {
    pub fn value(&self) -> preserves::IOValue {
        match self {
            Self::Returned(reference) => crate::codec::record("reply-returned", vec![crate::codec::string(reference)]),
            Self::Failed(failure) => crate::codec::record("reply-failed", vec![failure.value()]),
        }
    }
    pub fn capacity(&self) -> usize {
        match self {
            Self::Returned(reference) => reference.capacity(),
            Self::Failed(failure) => failure.retained_bytes(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Observation {
    pub terminal: Terminal,
    pub output: Output,
    pub hostcalls: Vec<Hostcall>,
    pub resource_class: String,
    pub final_state: String,
}

impl Observation {
    pub fn value(&self) -> preserves::IOValue {
        let calls = self
            .hostcalls
            .iter()
            .map(|call| {
                crate::codec::record("hostcall", vec![
                    crate::codec::string(&call.name),
                    crate::codec::string(&call.request),
                    call.response.value(),
                    crate::codec::string(&call.authority),
                ])
            })
            .collect();
        crate::codec::record("spacewasm-observation-v1", vec![
            self.terminal.value(),
            self.output.value(),
            crate::codec::sequence(calls),
            crate::codec::string(&self.resource_class),
            crate::codec::string(&self.final_state),
        ])
    }

    pub fn identity(&self) -> Result<String, String> {
        crate::codec::canonical_hash(&self.value()).map_err(|error| error.to_string())
    }

    pub fn validate(&self) -> Result<(), String> {
        crate::codec::validate_content_ref(&self.final_state).map_err(|e| e.to_string())?;
        if let Terminal::Failed(failure) = &self.terminal {
            if failure.is_proof_error() {
                return Err("harness/proof failure is not replay agreement".into());
            }
            if self.resource_class != failure.class() {
                return Err("terminal resource classification mismatch".into());
            }
        } else if !matches!(self.output, Output::Canonical(_)) || self.resource_class != "within-exact-profile" {
            return Err("successful terminal requires canonical output and admitted resources".into());
        }
        let malformed = self.terminal == Terminal::Failed(super::continuation::Failure::InvalidOutput);
        if malformed != matches!(self.output, Output::Rejected { .. }) {
            return Err("malformed-output terminal must retain the actual rejected bytes".into());
        }
        for (index, call) in self.hostcalls.iter().enumerate() {
            if !matches!(call.name.as_str(), "assert" | "send") {
                return Err("unknown terminal hostcall".into());
            }
            crate::codec::validate_content_ref(&call.request).map_err(|e| e.to_string())?;
            crate::codec::validate_content_ref(&call.authority).map_err(|e| e.to_string())?;
            match &call.response {
                Response::Returned(reference) => {
                    crate::codec::validate_content_ref(reference).map_err(|e| e.to_string())?;
                }
                Response::Failed(error) => {
                    if index + 1 != self.hostcalls.len() || self.terminal != Terminal::Failed(error.clone()) {
                        return Err("failed dispatch must be the exact terminal cause and last call".into());
                    }
                }
            }
        }
        match &self.output {
            Output::Absent => {}
            Output::Canonical(bytes) => {
                crate::codec::parse_canonical_bytes(bytes).map_err(|e| e.to_string())?;
            }
            Output::Rejected { bytes, issue } => {
                if issue.is_empty() || crate::codec::parse_canonical_bytes(bytes).is_ok() {
                    return Err("rejected output is not an actual canonical validation failure".into());
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Event {
    pub phase: String,
    pub binding: String,
    pub sequence: u64,
    pub fuel_granted: u64,
    pub detail: String,
    pub resource: Option<Resource>,
}

impl Event {
    pub fn value(&self) -> preserves::IOValue {
        crate::codec::record("spacewasm-event-v1", vec![
            crate::codec::string(&self.phase),
            crate::codec::string(&self.binding),
            crate::codec::u64_value(self.sequence),
            crate::codec::u64_value(self.fuel_granted),
            crate::codec::string(&self.detail),
            crate::codec::sequence(self.resource.as_ref().map(|resource| vec![resource.value()]).unwrap_or_default()),
        ])
    }
}

pub fn replay(expected: &Observation, actual: &Observation) -> Result<String, String> {
    expected.validate()?;
    actual.validate()?;
    let class = if expected == actual { "match" } else { "mismatch" };
    let value = crate::codec::record("spacewasm-replay-v1", vec![
        crate::codec::string(class),
        crate::codec::string(expected.identity()?),
        crate::codec::string(actual.identity()?),
        crate::codec::string("experimental-not-release-evidence"),
    ]);
    let identity = crate::codec::canonical_hash(&value).map_err(|error| error.to_string())?;
    if expected == actual {
        Ok(identity)
    } else {
        Err(identity)
    }
}

#[derive(Clone, Debug)]
pub struct Resource {
    pub interpreter_bytes: u64,
    pub interpreter_peak: u64,
    pub ir_bytes: u64,
    pub ir_peak: u64,
    pub stack_bytes: u64,
    pub linear_bytes: u64,
    pub instruction_bound: u64,
}

impl Resource {
    fn value(&self) -> preserves::IOValue {
        crate::codec::record("spacewasm-resource-v1", vec![
            crate::codec::u64_value(self.interpreter_bytes),
            crate::codec::u64_value(self.interpreter_peak),
            crate::codec::u64_value(self.ir_bytes),
            crate::codec::u64_value(self.ir_peak),
            crate::codec::u64_value(self.stack_bytes),
            crate::codec::u64_value(self.linear_bytes),
            crate::codec::u64_value(self.instruction_bound),
        ])
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Terminal {
    Finished,
    Failed(super::continuation::Failure),
}
impl Terminal {
    pub fn value(&self) -> preserves::IOValue {
        match self {
            Self::Finished => crate::codec::record("finished", Vec::new()),
            Self::Failed(failure) => crate::codec::record("failed", vec![failure.value()]),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Output {
    Absent,
    Canonical(Vec<u8>),
    Rejected { bytes: Vec<u8>, issue: String },
}
impl Output {
    pub fn bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Absent => None,
            Self::Canonical(bytes) | Self::Rejected { bytes, .. } => Some(bytes),
        }
    }
    pub fn capacity(&self) -> usize {
        match self {
            Self::Absent => 0,
            Self::Canonical(bytes) => bytes.capacity(),
            Self::Rejected { bytes, issue } => bytes.capacity() + issue.capacity(),
        }
    }
    pub fn value(&self) -> preserves::IOValue {
        match self {
            Self::Absent => crate::codec::record("output-absent", Vec::new()),
            Self::Canonical(bytes) => crate::codec::record("canonical-output", vec![
                crate::codec::string(crate::codec::content_ref_from_bytes(bytes)),
                crate::codec::u64_value(bytes.len() as u64),
            ]),
            Self::Rejected { bytes, issue } => crate::codec::record("rejected-output", vec![
                crate::codec::string(crate::codec::content_ref_from_bytes(bytes)),
                crate::codec::u64_value(bytes.len() as u64),
                crate::codec::string(issue),
            ]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation() -> Observation {
        let reference = |bytes: &[u8]| crate::codec::content_ref_from_bytes(bytes);
        let output = crate::codec::canonical_bytes(&crate::codec::record("answer", Vec::new())).unwrap();
        Observation {
            terminal: Terminal::Finished,
            output: Output::Canonical(output),
            hostcalls: vec![
                Hostcall {
                    name: "assert".into(),
                    request: reference(b"request-a"),
                    response: Response::Returned(reference(b"response-a")),
                    authority: reference(b"authority-a"),
                },
                Hostcall {
                    name: "send".into(),
                    request: reference(b"request-b"),
                    response: Response::Returned(reference(b"response-b")),
                    authority: reference(b"authority-b"),
                },
            ],
            resource_class: "within-exact-profile".into(),
            final_state: reference(b"state-a"),
        }
    }

    #[test]
    fn replay_rejects_reordered_effects_even_with_equal_output() {
        let expected = observation();
        let mut actual = expected.clone();
        actual.hostcalls.swap(0, 1);
        assert!(replay(&expected, &actual).is_err());
        assert_ne!(expected.identity().unwrap(), actual.identity().unwrap());
    }

    #[test]
    fn replay_requires_terminal_resource_and_final_state_agreement() {
        let expected = observation();
        assert!(replay(&expected, &expected).is_ok());
        let mut actual = expected.clone();
        actual.final_state = crate::codec::content_ref_from_bytes(b"state-b");
        assert!(replay(&expected, &actual).is_err());
        actual = expected.clone();
        actual.resource_class = "allocation-exhausted".into();
        assert!(replay(&expected, &actual).is_err());
        actual = expected.clone();
        actual.terminal = Terminal::Failed(super::super::continuation::Failure::Trap("unreachable".into()));
        assert!(replay(&expected, &actual).is_err());
    }
}

#[cfg(test)]
mod terminaltests {
    use super::*;

    fn failed(failure: super::super::continuation::Failure, output: Output) -> Observation {
        Observation {
            resource_class: failure.class().into(),
            terminal: Terminal::Failed(failure),
            output,
            hostcalls: Vec::new(),
            final_state: crate::codec::content_ref_from_bytes(b"observed-memory-and-globals"),
        }
    }

    #[test]
    fn rejected_output_is_not_absent_or_successful_output() {
        let observation = failed(super::super::continuation::Failure::InvalidOutput, Output::Rejected {
            bytes: vec![0xff],
            issue: "canonical-validation-failed".into(),
        });
        assert!(replay(&observation, &observation).is_ok());
        let mut absent = observation.clone();
        absent.output = Output::Absent;
        assert!(absent.validate().is_err());
        let mut other_bytes = observation.clone();
        other_bytes.output = Output::Rejected {
            bytes: vec![0xff, 0xff],
            issue: "canonical-validation-failed".into(),
        };
        assert!(other_bytes.validate().is_ok());
        assert!(replay(&observation, &other_bytes).is_err());
    }

    #[test]
    fn uncertainty_preserves_cause_failed_call_and_observed_state() {
        let cause = super::super::continuation::Failure::Uncertain("committed-assertion-response-lost".into());
        let mut observation = failed(cause.clone(), Output::Absent);
        observation.hostcalls.push(Hostcall {
            name: "assert".into(),
            request: crate::codec::content_ref_from_bytes(b"canonical-request"),
            response: Response::Failed(cause),
            authority: crate::codec::content_ref_from_bytes(b"verified-authority"),
        });
        assert!(replay(&observation, &observation).is_ok());
        let mut drift = observation.clone();
        drift.final_state = crate::codec::content_ref_from_bytes(b"different-observed-state");
        assert!(replay(&observation, &drift).is_err());
        drift = observation.clone();
        drift.hostcalls[0].response = Response::Failed(super::super::continuation::Failure::Trap("host".into()));
        assert!(drift.validate().is_err());
    }

    #[test]
    fn equal_harness_or_proof_errors_never_establish_replay_agreement() {
        for failure in [
            super::super::continuation::Failure::Harness("invalid-IR".into()),
            super::super::continuation::Failure::Proof("terminal-state-unavailable".into()),
        ] {
            let observation = failed(failure, Output::Absent);
            assert!(replay(&observation, &observation).is_err());
        }
    }
}
