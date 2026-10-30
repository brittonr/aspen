//! Leader-local public register operations: propose or read, then await the matching replica
//! observation.

impl super::Host {
    pub(in super::super) async fn perform(
        &mut self,
        request: &super::super::super::protocol::Request,
        is_write: bool,
    ) -> super::super::super::protocol::Response {
        let request_ref = match request_ref(request) {
            Ok(reference) => reference,
            Err(error) => return super::denied(request, error.to_string()),
        };
        let event = if is_write {
            let value = request
                .operation
                .as_ref()
                .and_then(|operation| operation.get("value"))
                .and_then(|value| value.get("value"))
                .and_then(|value| value.as_i64());
            let Some(value) = value else {
                return super::denied(request, "write-requires-admitted-integer".into());
            };
            let registry = self.replica.ports().application.handler();
            let command_ref = match registry.command(value) {
                Ok(reference) => reference,
                Err(error) => return super::denied(request, error.to_string()),
            };
            crate::fabric_consistency::raft::ReplicaEvent::Propose {
                request_ref: request_ref.clone(),
                command_ref,
                command_schema_ref: registry.schema_ref().to_string(),
            }
        } else {
            crate::fabric_consistency::raft::ReplicaEvent::Read {
                request_ref: request_ref.clone(),
                mode: crate::fabric_consistency::ReadMode::Linearizable,
            }
        };
        match self.replica.handle_event(event).await {
            crate::fabric_consistency::raft::ReplicaExecutionOutcome::Denied { diagnostic, .. } => {
                return super::denied(request, diagnostic);
            }
            crate::fabric_consistency::raft::ReplicaExecutionOutcome::Failed(failure) => {
                return super::super::super::protocol::uncertain(request, failure.diagnostic);
            }
            crate::fabric_consistency::raft::ReplicaExecutionOutcome::Applied(_) => {}
        }
        self.wait(request, &request_ref, is_write).await
    }

    async fn wait(
        &mut self,
        request: &super::super::super::protocol::Request,
        reference: &str,
        is_write: bool,
    ) -> super::super::super::protocol::Response {
        let expiry = tokio::time::sleep(std::time::Duration::from_millis(request.deadline_ms));
        tokio::pin!(expiry);
        loop {
            tokio::select! {
                observation = self.observations.recv() => {
                    let Some(observation) = observation else { return super::super::super::protocol::uncertain(request, "supervision-closed".into()); };
                    if let Some(outcome) = classify(&observation.kind, reference, is_write) {
                        return match outcome {
                            super::super::super::protocol::Outcome::Ok => super::success(request, if is_write { serde_json::Value::Null }
                                else { self.replica.ports().application.handler().value() }),
                            super::super::super::protocol::Outcome::Fail => super::denied(request, "replica-pre-effect-denial".into()),
                            super::super::super::protocol::Outcome::Info => super::super::super::protocol::uncertain(request, "replica-retryable".into()),
                        };
                    }
                }
                event = self.timers.recv() => {
                    let Some(event) = event else { return super::super::super::protocol::uncertain(request, "timer-closed".into()); };
                    if let Err(error) = self.apply(event).await { return super::super::super::protocol::uncertain(request, error.to_string()); }
                }
                event = self.ingress.next() => {
                    match event {
                        Ok(event) => if let Err(error) = self.apply(event.event).await { return super::super::super::protocol::uncertain(request, error.to_string()); },
                        Err(error) => return super::super::super::protocol::uncertain(request, error.to_string()),
                    }
                }
                _ = &mut expiry => return super::super::super::protocol::uncertain(request, "public-operation-deadline".into()),
            }
        }
    }
}

fn classify(
    observation: &crate::fabric_consistency::raft::ReplicaObservationKind,
    reference: &str,
    is_write: bool,
) -> Option<super::super::super::protocol::Outcome> {
    match observation {
        crate::fabric_consistency::raft::ReplicaObservationKind::Proposal {
            request_ref,
            disposition,
            ..
        } if is_write && request_ref == reference => Some(match disposition {
            crate::fabric_consistency::raft::ProposalDisposition::Committed => {
                super::super::super::protocol::Outcome::Ok
            }
            crate::fabric_consistency::raft::ProposalDisposition::Denied => {
                super::super::super::protocol::Outcome::Fail
            }
            crate::fabric_consistency::raft::ProposalDisposition::Retryable
            | crate::fabric_consistency::raft::ProposalDisposition::Cancelled
            | crate::fabric_consistency::raft::ProposalDisposition::Uncertain => {
                super::super::super::protocol::Outcome::Info
            }
        }),
        crate::fabric_consistency::raft::ReplicaObservationKind::Read {
            request_ref,
            disposition,
            ..
        } if !is_write && request_ref == reference => Some(match disposition {
            crate::fabric_consistency::raft::ReadDisposition::Current => super::super::super::protocol::Outcome::Ok,
            crate::fabric_consistency::raft::ReadDisposition::Denied => super::super::super::protocol::Outcome::Fail,
            crate::fabric_consistency::raft::ReadDisposition::Retryable
            | crate::fabric_consistency::raft::ReadDisposition::Local => super::super::super::protocol::Outcome::Info,
        }),
        _ => None,
    }
}

fn request_ref(request: &super::super::super::protocol::Request) -> crate::error::Result<String> {
    let operation = request.operation.as_ref();
    let field = |name: &str| operation.and_then(|value| value.get(name)).and_then(|value| value.as_str()).unwrap_or("");
    let value = operation.and_then(|value| value.get("value")).map(serde_json::Value::to_string).unwrap_or_default();
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("live-registry-client-request-v1", vec![
        crate::preserves_rail::string(&request.run_id),
        crate::preserves_rail::string(field("id")),
        crate::preserves_rail::string(field("process")),
        crate::preserves_rail::string(field("object_key")),
        crate::preserves_rail::string(field("function")),
        crate::preserves_rail::string(&value),
    ]))
}
