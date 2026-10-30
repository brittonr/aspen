//! Actual node-loop ownership of admitted native supervisors and journal locks.

pub(super) struct Session {
    host: Option<molten::node_service_host::installed::Owner>,
}

impl Session {
    pub(super) fn open(state_root: &std::path::Path) -> molten::error::Result<Self> {
        let root = molten::node_state::DirectoryRoot::open_existing(state_root)?;
        let gate = molten::node_daemon::admit_startup_source_gate_with_root(&root)?;
        let host = molten::node_service_host::installed::Owner::open_after_source_admission(&root, &gate)?;
        Ok(Self { host })
    }

    /// Publish only the current process's successfully authorized hosting owner.
    /// No persistent marker can be confused with a predecessor killed by a fault.
    pub(super) fn publish_ready(&self) -> molten::error::Result<()> {
        if let Some(host) = &self.host {
            println!("{}", molten::preserves_rail::to_text(host.ready())?);
            std::io::Write::flush(&mut std::io::stdout()).map_err(molten::error::Failure::from)?;
        }
        Ok(())
    }

    pub(super) fn close(self) {
        if let Some(host) = self.host {
            host.close();
        }
    }
}
