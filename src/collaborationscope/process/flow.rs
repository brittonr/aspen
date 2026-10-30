use super::*;

pub(super) struct Progress {
    step: u8,
    recovering: bool,
    first: Option<Command>,
    late: Option<Command>,
    replay: crate::collaboration_scope::NonceLedger,
    snapshot: Option<crate::collaboration_scope::ScopeAdmissionSnapshot>,
}
impl Progress {
    pub(super) fn new(replay: crate::collaboration_scope::NonceLedger, recovering: bool) -> Self {
        Self {
            step: 0,
            recovering,
            first: None,
            late: None,
            replay,
            snapshot: None,
        }
    }
    pub(super) async fn tick(
        &mut self,
        node: &mut live_cluster::Node,
        root: &std::path::Path,
    ) -> crate::error::Result<()> {
        match self.step {
            0 => {
                begin_read(node, "scope-current-first").await?;
                self.step = 1;
            }
            1 => {
                if let Some(current) = current(node)? {
                    if self.recovering {
                        let command = self.stage_add(node, current, "recovered-member")?;
                        propose(node, command).await?;
                        self.step = 10;
                    } else {
                        let snapshot = crate::collaboration_scope::fixtures::Signers::new().snapshot(
                            application(node)?,
                            current,
                            &self.replay,
                        )?;
                        write(
                            &root.join("prior-allowed-snapshot.preserves"),
                            &crate::collaboration_scope::snapshot_value(&snapshot),
                        )?;
                        self.snapshot = Some(snapshot);
                        begin_read(node, "scope-current-stage-first").await?;
                        self.step = 12;
                    }
                }
            }
            12 => {
                if let Some(current) = current(node)? {
                    self.first = Some(self.stage_add(node, current, "first-member")?);
                    begin_read(node, "scope-current-concurrent").await?;
                    self.step = 2;
                }
            }
            2 => {
                if let Some(current) = current(node)? {
                    let second = self.stage_add(node, current, "second-member")?;
                    propose(node, self.first.take().ok_or_else(|| invalid("first command missing"))?).await?;
                    propose(node, second).await?;
                    self.step = 3;
                }
            }
            3 if node.service.state().last_applied == 2 => {
                let app = application(node)?;
                if app.state().scopes[&crate::collaboration_scope::fixtures::space_ref()].membership_epoch != 2 {
                    return Err(invalid("conflict changed membership"));
                }
                let snapshot = self.snapshot.as_ref().ok_or_else(|| invalid("prior allowed snapshot missing"))?;
                if crate::collaboration_scope::revalidate_snapshot(
                    app.state(),
                    app.state_ref(),
                    snapshot,
                    &snapshot.decisions,
                )
                .is_ok()
                {
                    return Err(invalid("stale consumer snapshot admitted"));
                }
                begin_read(node, "scope-current-remove").await?;
                self.step = 4;
            }
            4 => {
                if let Some(current) = current(node)? {
                    let mutation = crate::collaboration_scope::Mutation::RemoveMember {
                        scope: crate::collaboration_scope::fixtures::space_ref(),
                        subject: crate::collaboration_scope::SubjectRef::new(
                            crate::collaboration_scope::fixtures::reference("subject-first-member"),
                        )
                        .map_err(|_| invalid("subject"))?,
                    };
                    let command = self.stage(node, current, mutation)?;
                    propose(node, command).await?;
                    self.step = 5;
                }
            }
            5 if node.service.state().last_applied == 3 => {
                begin_read(node, "scope-current-partition").await?;
                self.step = 6;
            }
            6 => {
                if let Some(current) = current(node)? {
                    self.late = Some(self.stage_add(node, current, "partitioned-member")?);
                    let application_state_ref = application(node)?.state_ref().as_str().to_string();
                    applied(node.service.handle_event(ReplicaEvent::CreateSnapshot { application_state_ref }).await)?;
                    applied(node.service.handle_event(ReplicaEvent::HeartbeatTimeout).await)?;
                    self.step = 7;
                }
            }
            7 if node.service.state().match_index.get("node-c").is_some_and(|index| *index >= 3) => {
                signal(&root.join("partition.preserves"));
                propose(node, self.late.take().ok_or_else(|| invalid("partition command missing"))?).await?;
                begin_read(node, "scope-partition-denied-currentness").await?;
                self.step = 8;
            }
            10 if node.service.state().last_applied == 4 => {
                applied(node.service.handle_event(ReplicaEvent::HeartbeatTimeout).await)?;
                self.step = 11;
            }
            _ => {}
        }
        Ok(())
    }
    pub(super) fn partition_pending(&self, node: &live_cluster::Node) -> bool {
        self.step == 8 && node.service.state().pending_reads.contains_key(&query("scope-partition-denied-currentness"))
    }
    fn stage_add(
        &self,
        node: &mut live_cluster::Node,
        current: crate::collaboration_scope::CurrentState,
        label: &str,
    ) -> crate::error::Result<Command> {
        let mutation = crate::collaboration_scope::fixtures::add_member(application(node)?.state(), label)?;
        self.stage(node, current, mutation)
    }
    fn stage(
        &self,
        node: &mut live_cluster::Node,
        current: crate::collaboration_scope::CurrentState,
        mutation: crate::collaboration_scope::Mutation,
    ) -> crate::error::Result<Command> {
        let application = node
            .service
            .ports_mut()
            .application
            .handler_mut()
            .collaboration
            .as_mut()
            .ok_or_else(|| invalid("application missing"))?;
        crate::collaboration_scope::fixtures::Signers::new().stage(application, current, mutation, &self.replay)
    }
}
fn application(node: &live_cluster::Node) -> crate::error::Result<&crate::collaboration_scope::Application> {
    node.service
        .ports()
        .application
        .handler()
        .collaboration
        .as_ref()
        .ok_or_else(|| invalid("application missing"))
}
fn query(label: &str) -> String {
    crate::fabric_consistency::raft::tests::test_ref(label)
}
async fn begin_read(node: &mut live_cluster::Node, label: &str) -> crate::error::Result<()> {
    applied(
        node.service
            .handle_event(ReplicaEvent::Read {
                request_ref: query(label),
                mode: crate::fabric_consistency::ReadMode::Linearizable,
            })
            .await,
    )
}
async fn propose(node: &mut live_cluster::Node, command: Command) -> crate::error::Result<()> {
    applied(
        node.service
            .handle_event(ReplicaEvent::Propose {
                request_ref: command.request_ref,
                command_ref: command.command_ref,
                command_schema_ref: command.command_schema_ref,
            })
            .await,
    )
}
fn current(node: &mut live_cluster::Node) -> crate::error::Result<Option<crate::collaboration_scope::CurrentState>> {
    while let Ok(observation) = node._control_receiver.try_recv() {
        if matches!(observation.kind, ReplicaObservationKind::Read {
            disposition: ReadDisposition::Current,
            ..
        }) {
            return crate::collaboration_scope::fixtures::current_from_replica(
                node.service.state(),
                node.service.evidence(),
                &observation,
                application(node)?,
            )
            .map(Some);
        }
    }
    Ok(None)
}
