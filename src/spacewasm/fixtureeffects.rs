//! Explicit isolated-fixture effects; never linked by normal actor selection.
use std::cell;

use super::authority;
use super::continuation;
use super::fixture;
use super::host;

pub(crate) struct Effects {
    pub scenario: fixture::Scenario,
    assertions: cell::RefCell<Vec<Vec<u8>>>,
}
impl Effects {
    pub fn new(scenario: fixture::Scenario) -> Self {
        Self {
            scenario,
            assertions: cell::RefCell::new(Vec::new()),
        }
    }
    pub fn response() -> Result<Vec<u8>, String> {
        crate::codec::canonical_bytes(&crate::codec::record("spacewasm-fixture-accepted-v1", Vec::new()))
            .map_err(|e| e.to_string())
    }
}
impl authority::EffectPort for Effects {
    fn perform(&self, name: &str, bytes: &[u8]) -> Result<host::Reply, continuation::Failure> {
        if !matches!(name, "assert" | "send") {
            return Err(continuation::Failure::Denied("unknown-fixture-effect".into()));
        }
        if let fixture::Scenario::HostTrap { reason } = &self.scenario {
            return Err(continuation::Failure::Trap(reason.clone()));
        }
        let response = Self::response().map_err(continuation::Failure::Harness)?;
        let mut entries = self.assertions.borrow_mut();
        if entries.len() >= 64 {
            return Err(continuation::Failure::Denied("fixture-effect-count".into()));
        }
        entries.try_reserve(1).map_err(|_| continuation::Failure::Allocation("fixture-effect-log".into()))?;
        let mut request = Vec::new();
        request
            .try_reserve_exact(bytes.len())
            .map_err(|_| continuation::Failure::Allocation("fixture-effect-bytes".into()))?;
        request.extend_from_slice(bytes);
        entries.push(request);
        if let fixture::Scenario::HostUncertain { reason } = &self.scenario {
            return Err(continuation::Failure::Uncertain(reason.clone()));
        }
        Ok(host::Reply {
            bytes: response,
            pause: self.scenario == fixture::Scenario::HostPause,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::authority;
    use super::super::continuation;
    use super::super::fixture;

    #[test]
    fn uncertainty_reports_failure_after_committing_exact_assertion() {
        let effects = super::Effects::new(fixture::Scenario::HostUncertain {
            reason: "after-commit".into(),
        });
        let request = crate::codec::canonical_bytes(&crate::codec::string("assertion")).unwrap();
        let result = authority::EffectPort::perform(&effects, "assert", &request);
        assert!(matches!(result, Err(continuation::Failure::Uncertain(reason)) if reason == "after-commit"));
        assert_eq!(*effects.assertions.borrow(), vec![request]);
    }

    #[test]
    fn host_trap_and_denied_effect_do_not_commit() {
        let effects = super::Effects::new(fixture::Scenario::HostTrap {
            reason: "before-commit".into(),
        });
        assert!(matches!(authority::EffectPort::perform(&effects, "assert", b"request"),
            Err(continuation::Failure::Trap(reason)) if reason == "before-commit"));
        assert!(effects.assertions.borrow().is_empty());
        assert!(matches!(
            authority::EffectPort::perform(&effects, "ambient", b"request"),
            Err(continuation::Failure::Denied(_))
        ));
        assert!(effects.assertions.borrow().is_empty());
    }
}
