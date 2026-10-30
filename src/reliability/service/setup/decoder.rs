#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    pub schema: String,
    pub node_id: String,
    pub group: super::consensus::Group,
    pub membership: super::consensus::Membership,
    pub replica: super::consensus::Replica,
    pub bindings: Vec<super::launch::PortBinding>,
    pub durability: Durability,
    pub transport: super::network::Transport,
    pub time: Time,
    pub registry: super::registry::RegistryAdmission,
    pub admission: super::runtime::HostAdmission,
    pub handler_ref: String,
    pub ingress_capacity: usize,
    pub ingress_delivery_limit: u64,
    pub health_path: String,
    pub peer_probes: std::collections::BTreeMap<String, super::leader::PeerProbe>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Durability {
    pub profile: super::profiles::DurableStateProfileDto,
    pub namespace: super::profiles::DurableNamespaceDescriptorDto,
    pub storage_namespace: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Time {
    pub profile: super::profiles::TimeProfileDescriptorDto,
    pub capability_ref: String,
    pub tick_ms: u64,
}

impl Configuration {
    pub fn load(
        authority: &super::access::Authorization,
        source: &crate::node_runtime::ValidatedNodeSourceGate,
    ) -> crate::error::Result<(Self, crate::fabric_consistency::Group)> {
        authority.grant.verify_current(&authority.root)?;
        let configuration: Self = serde_json::from_value(authority.grant.configuration().clone())
            .map_err(|error| crate::error::Failure::invalid_harness(error.to_string()))?;
        if configuration.schema != "molten.live-registry-service.v1"
            || configuration.bindings.len() != 7
            || !(1..=256).contains(&configuration.ingress_capacity)
            || !(1..=65_536).contains(&configuration.ingress_delivery_limit)
            || !(1..=30_000).contains(&configuration.time.tick_ms)
        {
            return Err(crate::error::Failure::invalid_harness("invalid bounded coordination service configuration"));
        }
        configuration.admission.validate()?;
        crate::node_state::RelativePath::parse(&configuration.health_path)?;
        let group = configuration.group.open(authority.grant.grant_ref())?;
        let grant = authority.grant.grant();
        if grant.node_id != configuration.node_id
            || grant.group_ref != group.binding_ref
            || grant.membership_ref != configuration.membership.membership_ref
            || grant.source_binding_ref != source.binding_ref()
            || grant.generation != group.service_generation
            || grant.endpoint != configuration.admission.endpoint.to_string()
            || configuration.replica.group_binding_ref != group.binding_ref
            || configuration.replica.protocol_ref.is_empty()
            || configuration.transport.protocol.service_id != group.service_id
            || configuration.transport.protocol.extension_id != group.extension_id
            || configuration.transport.protocol.generation != group.service_generation
            || configuration.durability.namespace.generation != group.service_generation
            || configuration.transport.peers.keys().any(|peer| !configuration.membership.voters.contains(peer))
        {
            return Err(crate::error::Failure::invalid_harness("service configuration differs from signed grant"));
        }
        Ok((configuration, group))
    }

    pub async fn assemble(
        self,
        authority: super::access::Authorization,
        source_gate: crate::node_runtime::ValidatedNodeSourceGate,
        group: crate::fabric_consistency::Group,
    ) -> crate::error::Result<super::runtime::Host> {
        authority.require("serve")?;
        let profile = self.durability.profile.into_profile()?;
        let namespace = self.durability.namespace.into_descriptor(&profile)?;
        let root = crate::local_store::DurableStoreRoot::from_ledger(
            &authority.root.ledger_store()?,
            &crate::local_store::Locator::parse(&self.durability.storage_namespace)?,
        )?;
        let adapter = crate::fabric_durability::RedbDurableStateAdapter::open_root(root, profile, namespace)?;
        let is_recovery = !adapter.state().durable_log.is_empty() || !adapter.state().snapshots.is_empty();
        let durability = crate::fabric_consistency::raft::RedbReplicaPort::new(
            adapter,
            self.replica.durable_log_ref.clone(),
            self.replica.snapshot_store_ref.clone(),
        )?;
        let transport = self.transport.open(&authority, &self.replica.protocol_ref).await?;
        let timer = self.time.profile.into_descriptor()?;
        super::launch::start(super::launch::Startup {
            group,
            node_id: self.node_id,
            membership: self.membership.decode(),
            profile: self.replica.decode(),
            bindings: self.bindings,
            durability,
            transport: transport.port,
            listener: transport.listener,
            ingress: crate::fabric_consistency::raft::ReplicaIngressConfig {
                session_ref: authority.grant.grant().transport_session_ref.clone(),
                accept_timeout: std::time::Duration::from_millis(self.transport.timeout_ms),
                event_capacity: self.ingress_capacity,
                delivery_limit: self.ingress_delivery_limit,
            },
            timer: crate::fabric_consistency::raft::TokioReplicaConfig {
                profile: timer.profile,
                generation: self.replica.service_generation,
                service_id: self.admission.profile.service.clone(),
                capability_ref: self.time.capability_ref,
                entropy_binding_ref: self.replica.entropy_profile_ref.clone(),
                tick_duration: std::time::Duration::from_millis(self.time.tick_ms),
                heartbeat_ticks: self.replica.heartbeat_ticks,
                election_min_ticks: self.replica.election_min_ticks,
                election_max_ticks: self.replica.election_max_ticks,
            },
            registry: super::registry::Registry::new(self.registry)?,
            handler_ref: self.handler_ref,
            host: self.admission,
            is_recovery,
            source_gate,
            authorization: authority,
            health_path: self.health_path,
            peer_probes: self.peer_probes,
            transport_registration: transport.registration,
        })
        .await
    }
}
