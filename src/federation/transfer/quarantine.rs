/// The capability must name quarantine, not the trusted artifact registry.
pub struct VerificationPort<'a> {
    pub staged: &'a std::cell::RefCell<Option<super::dagnetwork::StagedObject>>,
    pub root: &'a crate::chunk_store::CapabilityChunkRoot,
    pub profile: &'a molten_core::content_store_adapter::ContentAdapterProfile,
    pub policy_ref: &'a str,
    pub now: u64,
    pub deadline: u64,
    pub maximum_bytes: u64,
    pub completed: &'a std::cell::RefCell<Vec<QuarantinedObject>>,
    pub maximum_objects: usize,
    pub diagnostics: &'a crate::node_state::Scope,
}

pub struct QuarantinedObject {
    pub object_ref: molten_core::dag_sync::DagObjectRef,
    pub manifest_ref: String,
    pub verification_ref: String,
    pub content_evidence: preserves::IOValue,
    origin: crate::mesh::envelope::network::VerifiedRecord,
}
impl QuarantinedObject {
    pub fn origin(&self) -> &crate::mesh::envelope::network::VerifiedRecord {
        &self.origin
    }
}

// r[impl molten.federated_pull_runtime.receiver_pull]
impl crate::dag_sync::DagContentVerificationPort for VerificationPort<'_> {
    fn verify(
        &mut self,
        plan: &molten_core::dag_sync::DagSyncPlan,
        envelope: &crate::dag_sync::DagTransportEnvelope,
        authority_ref: &str,
    ) -> crate::error::Result<molten_core::dag_sync::DagResponseObservation> {
        crate::preserves_rail::validate_content_ref(authority_ref)?;
        if self.completed.borrow().len() >= self.maximum_objects
            || self.maximum_objects > molten_core::dag_sync::MAX_DAG_NODES
        {
            return Err(invalid("quarantine object bound exceeded"));
        }
        let staged = self.staged.borrow_mut().take().ok_or_else(|| invalid("no receiver-requested staged object"))?;
        if staged.object_ref != envelope.object_ref
            || staged.transport_ref != envelope.transport_observation_ref
            || staged.encoded_bytes != envelope.encoded_bytes
        {
            return Err(invalid("quarantine envelope correlation denied"));
        }
        let fields = staged
            .origin
            .record()
            .payload
            .collect_simple_record("federation-object-v1", Some(4))
            .ok_or_else(|| invalid("malformed staged response"))?;
        let manifest_value = crate::preserves_rail::value_to_iovalue(&fields[2]);
        let manifest = crate::chunk_store::parse_manifest_value(&manifest_value, None)?;
        let values = fields[3].collect_sequence().ok_or_else(|| invalid("object bytes must be a sequence"))?;
        if u64::try_from(values.len()).ok().is_none_or(|size| size > self.maximum_bytes)
            || u64::try_from(values.len()).ok() != Some(manifest.total_len)
        {
            return Err(invalid("federation object exceeds quarantine byte bound"));
        }
        let bytes = values
            .iter()
            .map(|value| {
                value
                    .as_u64()
                    .and_then(|value| value.ok())
                    .and_then(|value| u8::try_from(value).ok())
                    .ok_or_else(|| invalid("invalid object byte"))
            })
            .collect::<crate::error::Result<Vec<_>>>()?;
        if crate::preserves_rail::content_ref_from_bytes(&bytes) != envelope.object_ref.as_str() {
            crate::mesh::envelope::boundary::observe(
                self.diagnostics,
                crate::mesh::envelope::boundary::FailureClass::ContentIdentityRejected,
                plan.epoch_ref.as_str(),
                plan.generation,
                &staged.origin.transport().request_ref,
            )?;
            return Err(invalid("federation object canonical identity mismatch"));
        }
        let descriptor = crate::content_store_adapter::manifest_descriptor(&manifest);
        let command =
            crate::content_store_adapter::command(self.profile, crate::content_store_adapter::CommandInput {
                operation_ref: staged.transport_ref,
                operation: molten_core::content_store_adapter::ContentOperation::Import,
                manifest: &descriptor,
                range: None,
                submitted_tick: self.now,
                deadline_tick: self.deadline,
                retry_count: 0,
                cancelled: false,
                policy_refs: vec![self.policy_ref.to_string(), authority_ref.to_string()],
            })?;
        let execution =
            crate::content_store_adapter::execute_stream_put(crate::content_store_adapter::StreamPutInput {
                profile: self.profile,
                root: self.root,
                command: &command,
                expected_manifest: &descriptor,
                object_kind: &manifest.object_kind,
                bytes: &bytes,
            })?;
        self.completed.borrow_mut().push(QuarantinedObject {
            object_ref: envelope.object_ref.clone(),
            manifest_ref: execution.manifest.manifest_ref,
            verification_ref: execution.event.artifact_ref,
            content_evidence: execution.event.value,
            origin: staged.origin,
        });
        Ok(molten_core::dag_sync::DagResponseObservation {
            epoch_ref: plan.epoch_ref.clone(),
            generation: plan.generation,
            object_ref: envelope.object_ref.clone(),
            assigned_peer: envelope.assigned_peer.clone(),
            identity_verified: true,
            authorization_admitted: true,
            encoded_bytes: envelope.encoded_bytes,
        })
    }
}
fn invalid(message: &str) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}
