// SPDX-License-Identifier: AGPL-3.0-or-later
//! Explicit opt-in process fault injection for the same-host conformance fixture.
//! Never enabled by policy files, peer requests, or environment variables.
use molten_batch_application::Error;
#[derive(Clone, Default)]
pub struct ProcessFault(Option<String>);
impl ProcessFault {
    pub fn at(point: Option<String>) -> Result<Self, Error> {
        if point.as_deref().is_some_and(|p| {
            !matches!(
                p,
                "before-dispatch"
                    | "after-dispatch"
                    | "before-output-persist"
                    | "after-output-persist"
                    | "before-terminal-commit"
                    | "after-terminal-commit"
                    | "before-progress-commit"
                    | "after-progress-commit"
                    | "before-progress-reply"
                    | "after-progress-ack"
            )
        }) {
            return Err(Error::Invalid("unknown process fault point".into()));
        }
        Ok(Self(point))
    }
    pub(crate) fn hit(&self, point: &str) {
        if self.0.as_deref() == Some(point) {
            eprintln!("orchestrator injected-process-fault point={point}");
            // Deliberate abrupt process death: no graceful drain, destructor, or
            // synthesized stop evidence. Redb reopen/worker reconciliation owns recovery.
            std::process::exit(86);
        }
    }
}
