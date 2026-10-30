impl super::Driver {
    pub(super) fn new(parts: super::Parts) -> Self {
        let port = super::super::native::CommitPort::new(parts.sender.clone(), &parts.installation);
        Self {
            service: parts.service,
            ingress: parts.ingress,
            sender: parts.sender,
            control: parts.control,
            installation: parts.installation,
            timeout: parts.timeout,
            authority: parts.authority,
            port,
            is_uncertain: false,
        }
    }
    pub(super) fn application(&self) -> &super::super::Application {
        self.service.ports().application.handler()
    }
    pub(super) fn application_mut(&mut self) -> &mut super::super::Application {
        self.service.ports_mut().application.handler_mut()
    }
    pub(super) async fn apply(
        &mut self,
        event: crate::fabric_consistency::raft::ReplicaEvent,
    ) -> crate::error::Result<()> {
        if matches!(
            self.service.handle_event(event).await,
            crate::fabric_consistency::raft::ReplicaExecutionOutcome::Failed(_)
        ) {
            self.is_uncertain = true;
            return Err(super::super::denied(super::super::Denial::Uncertain));
        }
        Ok(())
    }
    pub(super) async fn step(
        &mut self,
        deadline: std::pin::Pin<&mut tokio::time::Sleep>,
    ) -> crate::error::Result<bool> {
        if self.is_uncertain {
            return Err(super::super::denied(super::super::Denial::Uncertain));
        }
        let event = tokio::select! {
            event = self.service.receive_event() => event?,
            incoming = self.ingress.next() => incoming?.event,
            _ = deadline => return Ok(false),
        };
        self.apply(event).await?;
        Ok(true)
    }
    pub(super) async fn commit(
        &mut self,
        binding: &crate::fabric::CanonicalPortBinding,
        effect: &crate::system_extension::TypedEffectRequest,
    ) -> crate::error::Result<crate::system_extension::PortEffectOutput> {
        if self.is_uncertain {
            return Err(super::super::denied(super::super::Denial::Uncertain));
        }
        let command = super::authority::restage(self, binding, effect).await?;
        self.installation.validate_replica(self.service.state())?;
        self.application().store.ensure_serving()?;
        self.port.enqueue(binding, effect, &command)?;
        let result = self.wait_for_commit(&command, effect).await;
        if result.is_err() {
            self.is_uncertain = true;
        }
        result
    }
    async fn wait_for_commit(
        &mut self,
        command: &crate::fabric_consistency::raft::Command,
        effect: &crate::system_extension::TypedEffectRequest,
    ) -> crate::error::Result<crate::system_extension::PortEffectOutput> {
        let deadline = tokio::time::sleep(self.timeout);
        tokio::pin!(deadline);
        loop {
            if let Some(proof) = super::proof::complete(self, command, effect)? {
                return Ok(proof);
            }
            if !self.step(deadline.as_mut()).await? {
                return Err(super::super::denied(super::super::Denial::Uncertain));
            }
        }
    }
}
