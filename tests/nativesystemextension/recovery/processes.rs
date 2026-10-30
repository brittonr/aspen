use crate::support::*;

pub(super) struct Child {
    owned: super::owned::Child,
}

impl Child {
    pub(super) fn spawn(command: std::process::Command) -> TestResult<Self> {
        let child = super::owned::spawn_piped(command).or_fail("spawn owned actual node process")?;
        Ok(Self {
            owned: super::owned::Child::adopt(child, 2_097_152),
        })
    }

    pub(super) fn await_record(&mut self, tag: &str) -> TestResult<preserves::IOValue> {
        let mut deadline = molten::fabric_time::SupervisionDeadline::after(std::time::Duration::from_secs(120))?;
        loop {
            let output = self.owned.output();
            if let Ok(value) = super::twin::value(&output, tag) {
                return Ok(value);
            }
            if self.owned.poll().is_some() || self.owned.is_overflowed() || deadline.is_expired()? {
                return Err(format!("actual hosted process did not reach admitted {tag}: {output}").into());
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    pub(super) fn kill(&mut self) -> TestResult<()> {
        if self.owned.poll().is_some() {
            return Err("node exited before injected process loss".into());
        }
        if !self.owned.stop() {
            return Err("owned node SIGKILL was not reaped".into());
        }
        if self.owned.poll().is_some_and(|status| status.success()) {
            return Err("injected process loss unexpectedly reported graceful success".into());
        }
        Ok(())
    }
}

impl Drop for Child {
    fn drop(&mut self) {
        self.owned.stop();
    }
}

pub(super) fn run(command: std::process::Command) -> TestResult<(bool, String)> {
    let mut child = Child::spawn(command)?;
    let mut deadline = molten::fabric_time::SupervisionDeadline::after(std::time::Duration::from_secs(120))?;
    loop {
        if let Some(status) = child.owned.poll() {
            child.owned.drain();
            if child.owned.is_overflowed() {
                return Err("actual node exceeded retained output bound".into());
            }
            return Ok((status.success(), child.owned.output()));
        }
        if child.owned.is_overflowed() || deadline.is_expired()? {
            return Err(
                format!("actual public node command exceeded supervision budget: {}", child.owned.output()).into()
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
