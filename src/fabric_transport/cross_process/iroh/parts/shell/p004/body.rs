impl Listener {
    // r[impl molten.fabric_transport.cross_process_listener]
    pub async fn drain_and_close(
        mut self,
        reason: ListenerDrainReason,
    ) -> crate::error::Result<ListenerCleanup> {
        self.state =
            apply_cross_process_listener_command(&self.state, &CrossProcessListenerCommand::BeginDrain { reason })
                .map_err(|issues| validation_error("cross-process listener drain", &issues))?
                .next;
        self.close_draining(reason).await
    }

    async fn close_draining(mut self, reason: ListenerDrainReason) -> crate::error::Result<ListenerCleanup> {
        if self.state.active_sessions != 0 {
            return Err(crate::error::Failure::invalid_harness(
                "cross-process listener drain requires all sessions to be terminal",
            ));
        }
        self.state = apply_cross_process_listener_command(&self.state, &CrossProcessListenerCommand::Close)
            .map_err(|issues| validation_error("cross-process listener close", &issues))?
            .next;
        self.endpoint.close().await;
        self.state = apply_cross_process_listener_command(&self.state, &CrossProcessListenerCommand::BeginCleanup)
            .map_err(|issues| validation_error("cross-process listener cleanup", &issues))?
            .next;
        let cleanup_evidence_ref = cleanup_ref(
            &self.state.identity.listener_identity_ref,
            &self.endpoint_artifact.descriptor_ref,
            self.protocol.generation,
        );
        self.state = apply_cross_process_listener_command(&self.state, &CrossProcessListenerCommand::CompleteCleanup {
            cleanup_evidence_ref: cleanup_evidence_ref.clone(),
        })
        .map_err(|issues| validation_error("cross-process listener cleanup completion", &issues))?
        .next;
        Ok(ListenerCleanup {
            listener_identity_ref: self.state.identity.listener_identity_ref.clone(),
            descriptor_ref: self.endpoint_artifact.descriptor_ref.clone(),
            generation: self.protocol.generation,
            drain_reason: reason,
            terminal_class: self.state.terminal_class.unwrap_or(ListenerTerminalClass::Clean),
            cleanup_evidence_ref,
        })
    }

    fn finish_listener_session(&mut self) -> crate::error::Result<()> {
        self.state = apply_cross_process_listener_command(&self.state, &CrossProcessListenerCommand::SessionTerminal {
            callback_generation: self.protocol.generation,
        })
        .map_err(|issues| validation_error("cross-process listener session terminal", &issues))?
        .next;
        Ok(())
    }
}
