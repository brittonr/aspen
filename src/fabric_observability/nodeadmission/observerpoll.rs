//! Read one copied Onix observation window through its separately admitted grant.

const MAX_CHECKED_PROFILE_BYTES: u64 = 64 * 1024;

/// Supplied by the node shell. The clock domains are deliberately distinct.
pub struct ObserverWindowTick<'a> {
    pub boot_id: &'a str,
    pub unix_seconds: u64,
    pub monotonic_ns: u64,
    pub deadline_ns: u64,
    pub cancelled: bool,
}

pub struct ObserverReadback {
    pub observation: crate::fabric_observability::EbpfServiceObservation,
    pub readiness: Option<ObserverReadiness>,
}

/// Only a complete required window can guard node readiness. It cannot poll again.
pub struct ObserverReadiness {
    authority: super::ObserverAuthorityLease,
    from_ns: u64,
    until_ns: u64,
}

impl ObserverReadiness {
    // r[impl linux_ebpf_observability.failure]
    pub fn validate(&self, unix_seconds: u64, monotonic_ns: u64) -> crate::error::Result<()> {
        if !(self.from_ns..=self.until_ns).contains(&monotonic_ns) {
            return Err(crate::error::Failure::invalid_harness("required Linux observation is not current"));
        }
        self.authority.validate_current(unix_seconds)
    }
}

impl super::ObserverAuthorityLease {
    /// Consume one signed epoch; any further poll requires a fresh admission.
    // r[impl linux_ebpf_observability.admission]
    // r[impl linux_ebpf_observability.boundary]
    pub fn poll_window(self, tick: ObserverWindowTick<'_>) -> crate::error::Result<ObserverReadback> {
        if tick.boot_id != self.selection.boot_id {
            return Err(denied());
        }
        self.validate_current(tick.unix_seconds)?;
        let (profile, adapter) = self.checked_profile()?;
        let directory = self.selection.endpoint_directory.strip_prefix("receipts/").ok_or_else(denied)?;
        let directory = crate::node_state::RelativePath::parse(directory).map_err(|_| denied())?;
        let endpoint = if cfg!(target_os = "linux") && !tick.cancelled && tick.monotonic_ns <= tick.deadline_ns {
            self.root
                .existing_namespace(crate::node_state::NamespaceKind::Receipts)?
                .map(|scope| scope.existing_subdir(&directory))
                .transpose()?
                .flatten()
        } else {
            None
        };
        let leaf = crate::node_state::RelativePath::parse(&self.selection.endpoint_leaf).map_err(|_| denied())?;
        let mut evidence_refs = vec![
            self.selection.source_validation_ref.clone(),
            self.selection.checked_profile_ref.clone(),
            self.selection.onix_receipt_ref.clone(),
        ];
        evidence_refs.sort_unstable();
        evidence_refs.dedup();
        let input = crate::fabric_observability::EbpfServiceInput {
            profile: &profile,
            adapter: &adapter,
            capability: endpoint.as_ref().map(|root| crate::fabric_observability::EbpfReadCapability {
                root,
                leaf,
                content_ref: Some(&self.selection.endpoint_content_ref),
            }),
            control: crate::fabric_observability::EbpfServiceControl {
                epoch: self.selection.epoch,
                now: tick.monotonic_ns,
                deadline: tick.deadline_ns,
                cancelled: tick.cancelled,
            },
            context: crate::fabric_observability::ObservationContext {
                source_id: "linux-ebpf".into(),
                source_ref: adapter.binding.adapter.clone(),
                profile_ref: profile.profile_ref.clone(),
                scope: crate::fabric_observability::ClaimScope::LocalComponent,
                generation: self.selection.epoch,
                observed_tick: tick.monotonic_ns,
                valid_until_tick: tick.monotonic_ns,
                resource_ref: adapter.binding.scope.clone(),
                evidence_refs,
                non_claims: crate::fabric_observability::REQUIRED_OBSERVABILITY_NON_CLAIMS.to_vec(),
            },
        };
        let result = crate::fabric_observability::observe_linux_ebpf(input)?;
        // Do not return a window if re-reading the verifier inputs observes withdrawal.
        self.validate_current(tick.unix_seconds)?;
        let readiness = (adapter.required && !result.deny_required_readiness).then(|| ObserverReadiness {
            authority: self,
            from_ns: tick.monotonic_ns,
            until_ns: result.health.artifact.context.valid_until_tick,
        });
        Ok(ObserverReadback {
            observation: result,
            readiness,
        })
    }

    fn checked_profile(
        &self,
    ) -> crate::error::Result<(crate::fabric_observability::ObservationProfile, crate::fabric_observability::EbpfProfile)>
    {
        let path =
            crate::node_state::RelativePath::parse(&self.selection.checked_profile_path).map_err(|_| denied())?;
        let bytes = self.root.read(&path, MAX_CHECKED_PROFILE_BYTES).map_err(|_| denied())?;
        if crate::preserves_rail::content_ref_from_bytes(&bytes) != self.selection.checked_profile_ref {
            return Err(denied());
        }
        let export = crate::fabric_observability::decode_ebpf_profile(&bytes).map_err(|_| denied())?;
        let profile = crate::fabric_observability::ebpf_observation_profile(&export).map_err(|_| denied())?;
        let descriptors = crate::fabric_observability::ebpf_metric_descriptors(&export).map_err(|_| denied())?;
        let adapter =
            crate::fabric_observability::checked_ebpf_profile(export, &profile, &descriptors).map_err(|_| denied())?;
        Ok((profile, adapter))
    }
}

fn denied() -> crate::error::Failure {
    crate::error::Failure::invalid_harness("Linux observer window selection denied")
}
