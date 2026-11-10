//! Bounded durable native-host bridge state, not a membership/authority store.
use super::*;

pub(super) struct HostState {
    generation: u64,
    incarnation: u64,
    sequence: u64,
    last_intent: Option<RequestRef>,
}
impl HostState {
    pub(super) fn observe(envelope: &crate::system_extension::DecodedCallbackEnvelope) -> crate::error::Result<Self> {
        let invocation = &envelope.invocation;
        let prior = envelope
            .inputs
            .state
            .as_ref()
            .map(|value| parse(&crate::preserves_rail::parse_canonical_bytes(&value.bytes)?))
            .transpose()?;
        match prior {
            None if invocation.callback != crate::system_extension::CallbackKind::Initialize => {
                Err(denied(Denial::Missing))
            }
            None => Ok(Self {
                generation: invocation.generation,
                incarnation: invocation.incarnation,
                sequence: invocation.sequence,
                last_intent: None,
            }),
            Some(mut state) => {
                if invocation.callback != crate::system_extension::CallbackKind::Recover {
                    if state.generation != invocation.generation || state.incarnation != invocation.incarnation {
                        return Err(denied(Denial::Stale));
                    }
                    if invocation.sequence <= state.sequence {
                        return Err(denied(Denial::Replay));
                    }
                }
                state.generation = invocation.generation;
                state.incarnation = invocation.incarnation;
                state.sequence = invocation.sequence;
                Ok(state)
            }
        }
    }
    pub(super) fn intent(&mut self, command: &crate::fabric_consistency::raft::Command) -> crate::error::Result<()> {
        let request = RequestRef::new(command.request_ref.clone()).map_err(|_| denied(Denial::Domain))?;
        self.last_intent = Some(request);
        Ok(())
    }
    pub(super) fn value(&self) -> preserves::IOValue {
        crate::preserves_rail::record("collaboration-native-bridge-v1", vec![
            crate::preserves_rail::string(SCHEMA),
            crate::preserves_rail::u64_value(self.generation),
            crate::preserves_rail::u64_value(self.incarnation),
            crate::preserves_rail::u64_value(self.sequence),
            crate::preserves_rail::record(
                "optional",
                self.last_intent.iter().map(|request| crate::preserves_rail::string(request.as_str())).collect(),
            ),
        ])
    }
    pub(super) fn status(&self) -> preserves::IOValue {
        crate::preserves_rail::record("collaboration-native-status-v1", vec![
            self.value(),
            crate::preserves_rail::string("proposal-intent-not-commit-or-authority"),
        ])
    }
}
fn parse(value: &preserves::IOValue) -> crate::error::Result<HostState> {
    let fields = decode::fields(value, "collaboration-native-bridge-v1", 5)?;
    if decode::string(&fields[0])? != SCHEMA {
        return Err(denied(Denial::Domain));
    }
    let state = HostState {
        generation: decode::integer(&fields[1])?,
        incarnation: decode::integer(&fields[2])?,
        sequence: decode::integer(&fields[3])?,
        last_intent: decode::option(&fields[4])?
            .map(RequestRef::new)
            .transpose()
            .map_err(|_| denied(Denial::Domain))?,
    };
    // Absence is carried by a missing state value, not by a zero field. The lifecycle authority's
    // first incarnation is zero and advances on restart, so a zero incarnation is a legitimate
    // first-instance identity and must not be read as a stale predecessor.
    if state.generation == 0 {
        return Err(denied(Denial::Stale));
    }
    Ok(state)
}
