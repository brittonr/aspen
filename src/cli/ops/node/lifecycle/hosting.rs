//! Actual node-loop ownership of admitted native supervisors and journal locks.

#[path = "control.rs"]
pub(super) mod control;

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

    pub(super) fn respond(&mut self, frame: &[u8]) -> preserves::IOValue {
        let request = match control::Request::decode(frame) {
            Ok(request) => request,
            Err(detail) => return control::error("invalid", 0, detail),
        };
        let Some(host) = self.host.as_mut() else {
            return control::error(request.command, request.tick, "native service hosting is not selected");
        };
        let value = match (request.mode, request.operation) {
            ("preview", control::Action::Host(operation)) => host
                .preview(&operation, request.tick)
                .map(|request| molten::node_service_host::command::request_value(&request)),
            (_, control::Action::Host(operation)) => host.execute(operation, request.tick),
            ("preview", control::Action::ScopeObserve { service, read }) => host
                .preview_scope(&service, &read, request.tick)
                .map(|request| molten::node_service_host::command::request_value(&request)),
            (_, control::Action::ScopeObserve { service, read }) => host.observe_scope(&service, &read, request.tick),
        };
        match value {
            Ok(value) => molten::preserves_rail::record("node-service-host-control-result-v1", vec![
                molten::preserves_rail::string(request.mode),
                molten::preserves_rail::string(request.command),
                molten::preserves_rail::u64_value(request.tick),
                value,
            ]),
            Err(error) => control::error(request.command, request.tick, control::failure_detail(&error)),
        }
    }

    pub(super) fn close(self) {
        if let Some(host) = self.host {
            host.close();
        }
    }
}
