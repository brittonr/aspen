pub struct Authority<'a> {
    pub proofset: &'a crate::capability_tokens::CapabilityProofset,
    pub head: &'a str,
}
impl crate::dag_sync::DagAuthorityPort for Authority<'_> {
    fn observe_authority(
        &mut self,
        plan: &molten_core::dag_sync::DagSyncPlan,
    ) -> crate::error::Result<crate::dag_sync::DagAuthorityObservation> {
        if plan.missing.iter().any(|object| object.as_str() != self.head) {
            return Err(super::super::invalid("DAG authority refuses objects outside exact local grant"));
        }
        let proof = self.proofset;
        let admitted =
            crate::capability_tokens::admit_capability(proof, &crate::capability_tokens::CapabilityRequest {
                holder_ref: proof.holder_ref.clone(),
                session_ref: proof.session_ref.clone(),
                context_ref: proof.context_ref.clone(),
                resource_ref: self.head.to_string(),
                ability: "federation:import".to_string(),
                scope: "local-registry".to_string(),
                at_tick: super::transport::NOW,
                required_policy_refs: proof.policy_refs.clone(),
                required_resource_refs: proof.resource_refs.clone(),
                required_token_kind: None,
                caveat_context: Vec::new(),
            })?;
        if admitted.decision != "pass" {
            return Err(super::super::invalid("DAG local capability denied"));
        }
        Ok(crate::dag_sync::DagAuthorityObservation {
            authority_ref: admitted.receipt_ref,
            plan_ref: plan.plan_ref.clone(),
            epoch_ref: plan.epoch_ref.clone(),
            generation: plan.generation,
            admitted: true,
        })
    }
}
pub struct Resources<'a> {
    pub scope: &'a crate::node_state::Scope,
    pub remaining: usize,
}
impl crate::dag_sync::DagResourcePort for Resources<'_> {
    fn reserve(
        &mut self,
        plan: &molten_core::dag_sync::DagSyncPlan,
    ) -> crate::error::Result<crate::dag_sync::DagResourceObservation> {
        self.remaining = self
            .remaining
            .checked_sub(plan.missing.len())
            .ok_or_else(|| super::super::invalid("DAG object reservation exhausted"))?;
        let value = crate::preserves_rail::record("federation-live-resource-reservation-v1", vec![
            crate::preserves_rail::string(plan.plan_ref.as_str()),
            crate::preserves_rail::u64_value(
                u64::try_from(plan.missing.len()).map_err(|_| super::super::invalid("reservation overflow"))?,
            ),
            crate::preserves_rail::u64_value(super::transport::LIMIT),
        ]);
        let reservation_ref = persist(self.scope, "reservation.preserves", &value)?;
        Ok(crate::dag_sync::DagResourceObservation {
            reservation_ref,
            plan_ref: plan.plan_ref.clone(),
            admitted: true,
        })
    }
}
pub struct Progress<'a> {
    pub scope: &'a crate::node_state::Scope,
}
impl crate::dag_sync::DagProgressPort for Progress<'_> {
    fn load(
        &mut self,
        _epoch: &molten_core::dag_sync::DagEpochRef,
    ) -> crate::error::Result<Option<molten_core::dag_sync::DagSyncProgress>> {
        if self.scope.try_exists(&crate::node_state::RelativePath::parse("dag-progress.preserves")?)? {
            return Err(super::super::invalid(
                "live fixture requires a fresh receiver; retained progress cannot be overwritten",
            ));
        }
        Ok(None)
    }
    fn store(&mut self, progress: &molten_core::dag_sync::DagSyncProgress) -> crate::error::Result<String> {
        persist(self.scope, "dag-progress.preserves", &crate::dag_sync::canonical_dag_progress(progress)?.value)
    }
}
pub struct Observations<'a> {
    pub scope: &'a crate::node_state::Scope,
}
impl crate::dag_sync::DagObservationPort for Observations<'_> {
    fn publish_response(&mut self, response: &crate::dag_sync::CanonicalDagRecord) -> crate::error::Result<()> {
        persist(self.scope, "dag-response.preserves", &response.value).map(|_| ())
    }
    fn publish_progress(&mut self, progress: &crate::dag_sync::CanonicalDagRecord) -> crate::error::Result<()> {
        persist(self.scope, "dag-observation.preserves", &progress.value).map(|_| ())
    }
}
pub struct Receipts<'a> {
    pub scope: &'a crate::node_state::Scope,
}
impl crate::dag_sync::DagReceiptPort for Receipts<'_> {
    fn publish_receipt(&mut self, receipt: &crate::dag_sync::CanonicalDagRecord) -> crate::error::Result<()> {
        persist(self.scope, "dag-receipt.preserves", &receipt.value).map(|_| ())
    }
}
pub fn persist(
    scope: &crate::node_state::Scope,
    leaf: &str,
    value: &preserves::IOValue,
) -> crate::error::Result<String> {
    scope.write(&crate::node_state::RelativePath::parse(leaf)?, &crate::preserves_rail::canonical_bytes(value)?)?;
    crate::preserves_rail::canonical_hash(value)
}
pub fn request(
    head: &str,
    session: &str,
    schema: &str,
    encoded_bytes: u64,
) -> crate::error::Result<(molten_core::dag_sync::DagGraph, molten_core::dag_sync::DagSyncRequest)> {
    let node = molten_core::dag_sync::DagNodeRef::new(head).map_err(|_| super::super::invalid("invalid node ref"))?;
    let schema =
        molten_core::dag_sync::DagSchemaRef::new(schema).map_err(|_| super::super::invalid("invalid schema ref"))?;
    let root = molten_core::dag_sync::DagRootRef::new(super::transport::reference("federation-live-root"))
        .map_err(|_| super::super::invalid("invalid root ref"))?;
    let graph = molten_core::dag_sync::DagGraph {
        roots: vec![molten_core::dag_sync::DagRoot {
            root_ref: root.clone(),
            domain: "federation-live".to_string(),
            node_ref: node.clone(),
            schema_ref: schema.clone(),
        }],
        nodes: vec![molten_core::dag_sync::DagNode {
            node_ref: node,
            schema_ref: schema,
            payload_ref: None,
            encoded_bytes,
            edges: Vec::new(),
        }],
    };
    let request = molten_core::dag_sync::DagSyncRequest {
        root_refs: vec![root],
        strategy: molten_core::dag_sync::DagSyncStrategy::PeerPartitioned,
        inventory: molten_core::dag_sync::DagInventory { available: Vec::new() },
        progress: None,
        peers: vec![
            molten_core::dag_sync::DagPeerId::new("federation-live-provider")
                .map_err(|_| super::super::invalid("invalid DAG peer"))?,
        ],
        epoch_ref: molten_core::dag_sync::DagEpochRef::new(session)
            .map_err(|_| super::super::invalid("invalid session ref"))?,
        generation: 1,
        policy_ref: molten_core::dag_sync::DagPolicyRef::new(super::transport::reference("federation-live-dag-policy"))
            .map_err(|_| super::super::invalid("invalid DAG policy"))?,
        bounds: molten_core::dag_sync::DagBounds {
            max_nodes: 1,
            max_edges: 1,
            max_roots: 1,
            max_depth: 1,
            max_bytes: super::transport::LIMIT,
            max_steps: 8,
            max_peers: 1,
        },
    };
    Ok((graph, request))
}
