#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DurableStateProfileDto {
    #[serde(deserialize_with = "super::bounded::text")]
    pub schema: String,
    #[serde(deserialize_with = "super::bounded::text")]
    pub profile_id: String,
    #[serde(deserialize_with = "super::bounded::text")]
    pub profile_ref: String,
    #[serde(deserialize_with = "super::bounded::text")]
    pub adapter_kind: String,
    #[serde(deserialize_with = "super::bounded::texts")]
    pub supported_levels: Vec<String>,
    pub max_namespaces: u64,
    pub max_log_records: u64,
    pub max_ordered_entries: u64,
    pub max_operation_bytes: u64,
    pub max_namespace_bytes: u64,
    pub max_batch_operations: u64,
    pub max_snapshots: u64,
    pub max_effect_transactions: u64,
    #[serde(deserialize_with = "super::bounded::texts")]
    pub non_claims: Vec<String>,
}

impl DurableStateProfileDto {
    fn into_raw(self) -> crate::error::Result<crate::fabric_durability::DurableStateProfile> {
        Ok(crate::fabric_durability::DurableStateProfile {
            schema: self.schema,
            profile_id: self.profile_id,
            profile_ref: self.profile_ref,
            adapter_kind: super::choice(
                &self.adapter_kind,
                &[
                    crate::fabric_durability::DurableAdapterKind::LiveRedb,
                    crate::fabric_durability::DurableAdapterKind::DeterministicSimulation,
                ],
                crate::fabric_durability::DurableAdapterKind::as_str,
            )?,
            supported_levels: super::choices(
                self.supported_levels,
                &[
                    crate::fabric_durability::DurabilityLevel::Buffered,
                    crate::fabric_durability::DurabilityLevel::ProcessLoss,
                    crate::fabric_durability::DurabilityLevel::MachineLoss,
                ],
                crate::fabric_durability::DurabilityLevel::as_str,
            )?,
            max_namespaces: self.max_namespaces,
            max_log_records: self.max_log_records,
            max_ordered_entries: self.max_ordered_entries,
            max_operation_bytes: self.max_operation_bytes,
            max_namespace_bytes: self.max_namespace_bytes,
            max_batch_operations: self.max_batch_operations,
            max_snapshots: self.max_snapshots,
            max_effect_transactions: self.max_effect_transactions,
            non_claims: super::choices(
                self.non_claims,
                &crate::fabric_durability::REQUIRED_DURABILITY_NON_CLAIMS,
                crate::fabric_durability::DurabilityNonClaim::as_str,
            )?,
        })
    }
    pub fn into_profile(self) -> crate::error::Result<crate::fabric_durability::DurableProfile> {
        crate::fabric_durability::durable_profile(&self.into_raw()?)
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AtomicityDomainDto {
    #[serde(deserialize_with = "super::bounded::text")]
    pub domain_id: String,
    #[serde(deserialize_with = "super::bounded::text")]
    pub adapter_id: String,
    #[serde(deserialize_with = "super::bounded::text")]
    pub namespace_id: String,
    pub generation: u64,
    #[serde(deserialize_with = "super::bounded::texts")]
    pub object_classes: Vec<String>,
    pub max_operations: u64,
    pub max_bytes: u64,
    #[serde(deserialize_with = "super::bounded::texts")]
    pub supported_levels: Vec<String>,
}

impl AtomicityDomainDto {
    fn into_raw(self) -> crate::error::Result<crate::fabric_durability::AtomicityDomain> {
        Ok(crate::fabric_durability::AtomicityDomain {
            domain_id: self.domain_id,
            adapter_id: self.adapter_id,
            namespace_id: self.namespace_id,
            generation: self.generation,
            object_classes: super::choices(
                self.object_classes,
                &[
                    crate::fabric_durability::DurableObjectClass::LogRecord,
                    crate::fabric_durability::DurableObjectClass::OrderedValue,
                    crate::fabric_durability::DurableObjectClass::Snapshot,
                    crate::fabric_durability::DurableObjectClass::Checkpoint,
                    crate::fabric_durability::DurableObjectClass::EffectTransaction,
                ],
                crate::fabric_durability::DurableObjectClass::as_str,
            )?,
            max_operations: self.max_operations,
            max_bytes: self.max_bytes,
            supported_levels: super::choices(
                self.supported_levels,
                &[
                    crate::fabric_durability::DurabilityLevel::Buffered,
                    crate::fabric_durability::DurabilityLevel::ProcessLoss,
                    crate::fabric_durability::DurabilityLevel::MachineLoss,
                ],
                crate::fabric_durability::DurabilityLevel::as_str,
            )?,
        })
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DurableNamespaceDescriptorDto {
    #[serde(deserialize_with = "super::bounded::text")]
    pub schema: String,
    #[serde(deserialize_with = "super::bounded::text")]
    pub profile_ref: String,
    #[serde(deserialize_with = "super::bounded::text")]
    pub adapter_id: String,
    #[serde(deserialize_with = "super::bounded::text")]
    pub namespace_id: String,
    pub generation: u64,
    #[serde(deserialize_with = "super::bounded::text")]
    pub value_schema_ref: String,
    pub atomicity_domain: AtomicityDomainDto,
    #[serde(deserialize_with = "super::bounded::optional_text")]
    pub retention_authority_ref: Option<String>,
    pub quota_bytes: u64,
}

impl DurableNamespaceDescriptorDto {
    fn into_raw(self) -> crate::error::Result<crate::fabric_durability::DurableNamespaceDescriptor> {
        Ok(crate::fabric_durability::DurableNamespaceDescriptor {
            schema: self.schema,
            profile_ref: self.profile_ref,
            adapter_id: self.adapter_id,
            namespace_id: self.namespace_id,
            generation: self.generation,
            value_schema_ref: self.value_schema_ref,
            atomicity_domain: self.atomicity_domain.into_raw()?,
            retention_authority_ref: self.retention_authority_ref,
            quota_bytes: self.quota_bytes,
        })
    }
    pub fn into_descriptor(
        self,
        profile: &crate::fabric_durability::DurableProfile,
    ) -> crate::error::Result<crate::fabric_durability::DurableNamespaceDescriptor> {
        let descriptor = self.into_raw()?;
        crate::fabric_durability::validate_namespace_descriptor(&profile.profile, &descriptor)
            .map_err(|issues| super::invalid(format!("invalid durable namespace: {issues:?}")))?;
        Ok(descriptor)
    }
}
