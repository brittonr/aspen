//! Read one copied Onix observation window through its separately admitted grant.
use crate::fabric_observability as observation;
use crate::node_state::NamespaceKind;
use crate::node_state::RelativePath;

const MAX_CHECKED_PROFILE_BYTES: u64 = 64 * 1024;

/// Supplied by the node shell. The clock domains are deliberately distinct.
pub struct ObserverWindowTick<'a> {
    pub boot_id: &'a str,
    pub unix_seconds: u64,
    pub monotonic_ns: u64,
    pub deadline_ns: u64,
    pub cancelled: bool,
}

impl super::ObserverAuthorityLease {
    /// Consume one signed epoch; any further poll requires a fresh admission.
    // r[impl linux_ebpf_observability.admission]
    // r[impl linux_ebpf_observability.boundary]
    pub fn poll_window(
        self,
        tick: ObserverWindowTick<'_>,
    ) -> crate::error::Result<observation::EbpfServiceObservation> {
        if tick.boot_id != self.selection.boot_id {
            return Err(denied());
        }
        self.validate_current(tick.unix_seconds)?;
        let (profile, adapter) = self.checked_profile()?;
        let directory = self.selection.endpoint_directory.strip_prefix("receipts/").ok_or_else(denied)?;
        let directory = RelativePath::parse(directory).map_err(|_| denied())?;
        let endpoint = if cfg!(target_os = "linux") && !tick.cancelled && tick.monotonic_ns <= tick.deadline_ns {
            self.root
                .existing_namespace(NamespaceKind::Receipts)?
                .map(|scope| scope.existing_subdir(&directory))
                .transpose()?
                .flatten()
        } else {
            None
        };
        let leaf = RelativePath::parse(&self.selection.endpoint_leaf).map_err(|_| denied())?;
        let mut evidence_refs = vec![
            self.selection.source_validation_ref.clone(),
            self.selection.checked_profile_ref.clone(),
            self.selection.onix_receipt_ref.clone(),
        ];
        evidence_refs.sort_unstable();
        evidence_refs.dedup();
        let result = observation::observe_linux_ebpf(observation::EbpfServiceInput {
            profile: &profile,
            adapter: &adapter,
            capability: endpoint.as_ref().map(|root| observation::EbpfReadCapability {
                root,
                leaf,
                content_ref: Some(&self.selection.endpoint_content_ref),
            }),
            control: observation::EbpfServiceControl {
                epoch: self.selection.epoch,
                now: tick.monotonic_ns,
                deadline: tick.deadline_ns,
                cancelled: tick.cancelled,
            },
            context: observation::ObservationContext {
                source_id: "linux-ebpf".into(),
                source_ref: adapter.binding.adapter.clone(),
                profile_ref: profile.profile_ref.clone(),
                scope: observation::ClaimScope::LocalComponent,
                generation: self.selection.epoch,
                observed_tick: tick.monotonic_ns,
                valid_until_tick: tick.monotonic_ns,
                resource_ref: adapter.binding.scope.clone(),
                evidence_refs,
                non_claims: observation::REQUIRED_OBSERVABILITY_NON_CLAIMS.to_vec(),
            },
        })?;
        // Do not return a window if re-reading the verifier inputs observes withdrawal.
        self.validate_current(tick.unix_seconds)?;
        Ok(result)
    }

    fn checked_profile(&self) -> crate::error::Result<(observation::ObservationProfile, observation::EbpfProfile)> {
        let path = RelativePath::parse(&self.selection.checked_profile_path).map_err(|_| denied())?;
        let bytes = self.root.read(&path, MAX_CHECKED_PROFILE_BYTES).map_err(|_| denied())?;
        if crate::preserves_rail::content_ref_from_bytes(&bytes) != self.selection.checked_profile_ref {
            return Err(denied());
        }
        let export = observation::decode_ebpf_profile(&bytes).map_err(|_| denied())?;
        let profile = observation::ebpf_observation_profile(&export).map_err(|_| denied())?;
        let descriptors = observation::ebpf_metric_descriptors(&export).map_err(|_| denied())?;
        let adapter = observation::checked_ebpf_profile(export, &profile, &descriptors).map_err(|_| denied())?;
        Ok((profile, adapter))
    }
}

fn denied() -> crate::error::Failure {
    crate::error::Failure::invalid_harness("Linux observer window selection denied")
}
