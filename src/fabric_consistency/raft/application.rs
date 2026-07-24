use std::collections::BTreeSet;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;

pub trait CommittedBatchHandler {
    fn apply_batch(&mut self, entries: &[ReplicatedEntry]) -> Result<String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaApplicationConfig {
    pub application_manifest_ref: String,
    pub handler_ref: String,
    pub command_schema_refs: BTreeSet<String>,
    pub initial_applied_index: u64,
}

pub struct AdmittedReplicaApplicationPort<H: CommittedBatchHandler> {
    config: ReplicaApplicationConfig,
    last_applied_index: u64,
    handler: H,
}

impl<H: CommittedBatchHandler> AdmittedReplicaApplicationPort<H> {
    pub fn new(config: ReplicaApplicationConfig, handler: H) -> Result<Self> {
        validate_application_config(&config)?;
        Ok(Self {
            last_applied_index: config.initial_applied_index,
            config,
            handler,
        })
    }

    pub const fn last_applied_index(&self) -> u64 {
        self.last_applied_index
    }

    pub fn application_manifest_ref(&self) -> &str {
        &self.config.application_manifest_ref
    }

    pub const fn handler(&self) -> &H {
        &self.handler
    }
}

impl<H: CommittedBatchHandler> ReplicaApplicationEffects for AdmittedReplicaApplicationPort<H> {
    fn apply_committed(&mut self, entries: &[ReplicatedEntry]) -> Result<String> {
        let plan = plan_application_batch(self.last_applied_index, &self.config.command_schema_refs, entries)?;
        let handler_evidence_ref = self.handler.apply_batch(entries)?;
        crate::preserves_rail::validate_content_ref(&handler_evidence_ref)?;
        let receipt_ref = application_receipt_ref(&self.config, &plan, entries, &handler_evidence_ref)?;
        self.last_applied_index = plan.last_index;
        Ok(receipt_ref)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ApplicationBatchPlan {
    first_index: u64,
    last_index: u64,
}

fn validate_application_config(config: &ReplicaApplicationConfig) -> Result<()> {
    crate::preserves_rail::validate_content_ref(&config.application_manifest_ref)?;
    crate::preserves_rail::validate_content_ref(&config.handler_ref)?;
    if config.command_schema_refs.is_empty() {
        return Err(MoltenError::invalid_harness(
            "live Raft application port requires at least one admitted command schema",
        ));
    }
    for schema_ref in &config.command_schema_refs {
        crate::preserves_rail::validate_content_ref(schema_ref)?;
    }
    Ok(())
}

fn plan_application_batch(
    last_applied_index: u64,
    command_schema_refs: &BTreeSet<String>,
    entries: &[ReplicatedEntry],
) -> Result<ApplicationBatchPlan> {
    let first = entries
        .first()
        .ok_or_else(|| MoltenError::invalid_harness("live Raft application port denies an empty committed batch"))?;
    let expected_first = last_applied_index
        .checked_add(NEXT_LOG_INDEX_STEP)
        .ok_or_else(|| MoltenError::invalid_harness("live Raft application index overflow"))?;
    if first.index != expected_first {
        return Err(MoltenError::invalid_harness("live Raft application batch is duplicated, stale, or noncontiguous"));
    }
    let mut expected_index = expected_first;
    for entry in entries {
        if entry.index != expected_index {
            return Err(MoltenError::invalid_harness("live Raft application batch contains an index gap"));
        }
        if !command_schema_refs.contains(&entry.command_schema_ref) {
            return Err(MoltenError::invalid_harness("live Raft application command schema is not admitted"));
        }
        for reference in [&entry.request_ref, &entry.command_ref, &entry.command_schema_ref] {
            crate::preserves_rail::validate_content_ref(reference)?;
        }
        expected_index = expected_index
            .checked_add(NEXT_LOG_INDEX_STEP)
            .ok_or_else(|| MoltenError::invalid_harness("live Raft application index overflow"))?;
    }
    let last_index = entries
        .last()
        .ok_or_else(|| MoltenError::invalid_harness("live Raft application batch became empty"))?
        .index;
    Ok(ApplicationBatchPlan {
        first_index: first.index,
        last_index,
    })
}

fn application_receipt_ref(
    config: &ReplicaApplicationConfig,
    plan: &ApplicationBatchPlan,
    entries: &[ReplicatedEntry],
    handler_evidence_ref: &str,
) -> Result<String> {
    let entry_refs = entries
        .iter()
        .map(|entry| {
            crate::preserves_rail::record("entry", vec![
                crate::preserves_rail::u64_value(entry.index),
                crate::preserves_rail::string(&entry.request_ref),
                crate::preserves_rail::string(&entry.command_ref),
                crate::preserves_rail::string(&entry.command_schema_ref),
            ])
        })
        .collect();
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("raft-application-batch-v1", vec![
        crate::preserves_rail::string(&config.application_manifest_ref),
        crate::preserves_rail::string(&config.handler_ref),
        crate::preserves_rail::u64_value(plan.first_index),
        crate::preserves_rail::u64_value(plan.last_index),
        crate::preserves_rail::sequence(entry_refs),
        crate::preserves_rail::string(handler_evidence_ref),
    ]))
}
