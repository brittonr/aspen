use std::cell;
use std::rc;

use super::continuation;
use super::host;

#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub request: Vec<u8>,
    pub response: RecordedResponse,
    pub authority: String,
}

#[derive(Clone, Debug)]
pub enum RecordedResponse {
    Returned { bytes: Vec<u8>, pause: bool },
    Failed(continuation::Failure),
}

/// Records actual admitted host responses, not a substitute for live authorization.
pub struct Recording {
    inner: rc::Rc<dyn host::HostPort>,
    entries: cell::RefCell<Vec<Entry>>,
    maximum: usize,
}

impl Recording {
    pub fn new(inner: rc::Rc<dyn host::HostPort>, maximum: usize) -> Self {
        Self {
            inner,
            entries: cell::RefCell::new(Vec::new()),
            maximum,
        }
    }
    pub fn entries(&self) -> Vec<Entry> {
        self.entries.borrow().clone()
    }
}

impl host::HostPort for Recording {
    fn dispatch(
        &self,
        binding: &continuation::Binding,
        name: &str,
        bytes: &[u8],
    ) -> Result<host::Reply, continuation::Failure> {
        let mut entries = self.entries.borrow_mut();
        if entries.len() >= self.maximum {
            return Err(continuation::Failure::Denied("effect-record-bound".into()));
        }
        entries.try_reserve(1).map_err(|_| continuation::Failure::Allocation("effect-record".into()))?;
        let request = bytes.to_vec();
        let authority = binding.authority.clone();
        let result = self.inner.dispatch(binding, name, bytes);
        let response = match &result {
            Ok(reply) => RecordedResponse::Returned {
                bytes: reply.bytes.clone(),
                pause: reply.pause,
            },
            Err(error) => RecordedResponse::Failed(error.clone()),
        };
        entries.push(Entry {
            name: name.into(),
            request,
            response,
            authority,
        });
        result
    }
}

/// No live effect port exists in this type. Order and complete consumption are mandatory.
pub struct Replay {
    entries: Vec<Entry>,
    next: cell::Cell<usize>,
}

impl Replay {
    pub fn new(entries: Vec<Entry>) -> Self {
        Self {
            entries,
            next: cell::Cell::new(0),
        }
    }
    pub fn finish(&self) -> Result<(), String> {
        if self.next.get() == self.entries.len() {
            Ok(())
        } else {
            Err("unconsumed replay effects".into())
        }
    }
}

// r[impl molten.spacewasm_core.receipts]
impl host::HostPort for Replay {
    fn dispatch(
        &self,
        binding: &continuation::Binding,
        name: &str,
        bytes: &[u8],
    ) -> Result<host::Reply, continuation::Failure> {
        let entry = self
            .entries
            .get(self.next.get())
            .ok_or_else(|| continuation::Failure::Denied("missing replay effect".into()))?;
        if entry.name != name || entry.request != bytes || entry.authority != binding.authority {
            return Err(continuation::Failure::Denied("reordered or cross-authority replay effect".into()));
        }
        self.next.set(self.next.get() + 1);
        match &entry.response {
            RecordedResponse::Returned { bytes, pause } => Ok(host::Reply {
                bytes: bytes.clone(),
                pause: *pause,
            }),
            RecordedResponse::Failed(error) => Err(error.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_rejects_cross_call_and_incomplete_logs() {
        let replay = Replay::new(vec![Entry {
            name: "assert".into(),
            request: vec![1],
            response: RecordedResponse::Returned {
                bytes: vec![2],
                pause: false,
            },
            authority: "authority".into(),
        }]);
        let binding = continuation::Binding {
            actor: "actor".into(),
            generation: 1,
            artifact: "module".into(),
            profile: "profile".into(),
            input: "input".into(),
            authority: "authority".into(),
            resource: "resource".into(),
            effects: "effects".into(),
        };
        assert!(host::HostPort::dispatch(&replay, &binding, "send", &[1]).is_err());
        assert!(host::HostPort::dispatch(&replay, &binding, "assert", &[3]).is_err());
        assert!(replay.finish().is_err());
    }
}

#[cfg(test)]
mod uncertainreplay {
    use std::rc;

    use super::super::authority;
    use super::super::continuation;
    use super::super::fixture;
    use super::super::fixtureauthority;
    use super::super::fixtureeffects;
    use super::super::host;

    #[test]
    fn signed_committed_uncertainty_is_recorded_and_replayed_without_live_effects() {
        let reference = |bytes: &[u8]| crate::codec::content_ref_from_bytes(bytes);
        let request = crate::codec::canonical_bytes(&crate::codec::string("assertion")).unwrap();
        let binding = continuation::Binding {
            actor: "fixture".into(),
            generation: 1,
            artifact: reference(b"module"),
            profile: reference(b"profile"),
            input: reference(&request),
            authority: reference(b"unissued"),
            resource: reference(b"resource"),
            effects: reference(b"effect-plan"),
        };
        let source = rc::Rc::new(fixtureauthority::Authority::new(binding).unwrap());
        let binding = continuation::CurrentAdmission::current(&*source).unwrap();
        let effects = rc::Rc::new(fixtureeffects::Effects::new(fixture::Scenario::HostUncertain {
            reason: "after-commit".into(),
        }));
        let live = rc::Rc::new(authority::AuthorizedHost {
            authority: source,
            effects,
        });
        let recording = super::Recording::new(live, 1);
        assert!(matches!(host::HostPort::dispatch(&recording, &binding, "assert", &request),
            Err(continuation::Failure::Uncertain(reason)) if reason == "after-commit"));
        let replay = super::Replay::new(recording.entries());
        assert!(matches!(host::HostPort::dispatch(&replay, &binding, "assert", &request),
            Err(continuation::Failure::Uncertain(reason)) if reason == "after-commit"));
        assert_eq!(replay.finish(), Ok(()));
        assert!(matches!(host::HostPort::dispatch(&replay, &binding, "assert", &request),
            Err(continuation::Failure::Denied(reason)) if reason == "missing replay effect"));
    }
}
