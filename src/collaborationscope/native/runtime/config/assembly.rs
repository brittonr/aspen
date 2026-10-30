pub(super) fn timer(
    input: &super::Input,
    profile: crate::fabric_time::AdmittedTimeProfile,
    sender: tokio::sync::mpsc::Sender<crate::fabric_consistency::raft::ReplicaEvent>,
) -> crate::error::Result<
    crate::fabric_consistency::raft::TokioReplicaPort<crate::fabric_time::OperatingSystemEntropySource>,
> {
    crate::fabric_consistency::raft::TokioReplicaPort::new_operating_system(
        crate::fabric_consistency::raft::TokioReplicaConfig {
            profile,
            generation: input.group.service_generation,
            service_id: input.group.service_id.clone(),
            capability_ref: input.time.capability_ref.clone(),
            entropy_binding_ref: input.replica.entropy_profile_ref.clone(),
            tick_duration: std::time::Duration::from_millis(input.time.tick_ms),
            heartbeat_ticks: input.replica.heartbeat_ticks,
            election_min_ticks: input.replica.election_min_ticks,
            election_max_ticks: input.replica.election_max_ticks,
        },
        sender,
    )
}

pub(super) fn control(
    input: &super::Input,
) -> crate::error::Result<(
    crate::fabric_consistency::raft::ChannelReplicaPort,
    tokio::sync::mpsc::Receiver<crate::fabric_consistency::raft::ReplicaObservation>,
)> {
    let (sender, receiver) = tokio::sync::mpsc::channel(256);
    let port = crate::fabric_consistency::raft::ChannelReplicaPort::new(
        crate::fabric_consistency::raft::ReplicaConfig {
            service_id: input.group.service_id.clone(),
            service_generation: input.group.service_generation,
            supervision_ref: input.replica.supervision_ref.clone(),
        },
        sender,
    )?;
    Ok((port, receiver))
}

pub(super) fn application(
    input: &super::Input,
    application: super::super::super::Application,
) -> crate::error::Result<crate::fabric_consistency::raft::AdmittedReplicaPort<super::super::super::Application>> {
    let initial_applied_index = application.state().currentness.committed_index;
    crate::fabric_consistency::raft::AdmittedReplicaPort::new(
        crate::fabric_consistency::raft::ServiceReplicaConfig {
            group_binding_ref: input.replica.group_binding_ref.clone(),
            application_manifest_ref: input.group.application_manifest_ref.clone(),
            handler_ref: input.handler_ref.clone(),
            command_schema_refs: [super::super::super::command_schema_ref()?].into(),
            initial_applied_index,
        },
        application,
    )
}

pub(super) fn ingress(
    input: &super::Input,
    listener: crate::fabric_transport::Listener,
) -> crate::error::Result<crate::fabric_consistency::raft::ReplicaIngressPump> {
    crate::fabric_consistency::raft::ReplicaIngressPump::spawn(
        listener,
        crate::fabric_consistency::raft::ReplicaIngressConfig {
            session_ref: input.session_ref.clone(),
            accept_timeout: std::time::Duration::from_millis(input.transport.timeout_ms),
            event_capacity: input.ingress_capacity,
            delivery_limit: input.ingress_delivery_limit,
        },
    )
}
