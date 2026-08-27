use std::path::Path;

use molten_core::world_distribution::WorldRetentionReport;

use super::CanonicalWorldDistributionRecord;
use super::canonical_world_retention_report;
use crate::error::Result;
use crate::retention::DestructiveEvidence;
use crate::retention::GcPlan;
use crate::retention::GcPlanInput;
use crate::retention::store_gc_plan;

const WORLD_DISTRIBUTION_SUBSYSTEM: &str = "world-distribution";

pub struct WorldRetentionHandoffInput<'a> {
    pub retention_root: &'a Path,
    pub report: &'a WorldRetentionReport,
    pub object_ref: &'a str,
    pub object_kind: &'a str,
    pub retention_class: &'a str,
    pub action: &'a str,
    pub existing_evidence: &'a DestructiveEvidence,
}

#[derive(Debug, Clone)]
pub struct WorldRetentionHandoff {
    pub report: CanonicalWorldDistributionRecord,
    pub plan: GcPlan,
    pub report_granted_deletion_authority: bool,
}

// r[impl molten.world_distribution.gc_boundary]
pub fn handoff_world_retention(input: WorldRetentionHandoffInput<'_>) -> Result<WorldRetentionHandoff> {
    let report = canonical_world_retention_report(input.report)?;
    let mut evidence = input.existing_evidence.clone();
    evidence.retained_refs.extend(input.report.retained_refs.iter().cloned());
    evidence.remote_refs.extend(input.report.remote_refs.iter().cloned());
    evidence.evidence_refs.extend(input.report.evidence_refs.iter().cloned());
    evidence.evidence_refs.push(report.record_ref.clone());
    evidence.reference_index_refs.push(report.record_ref.clone());
    evidence.remote_gc_refs.extend(input.report.unresolved_remote.iter().cloned());
    evidence.is_reference_index_complete =
        evidence.is_reference_index_complete && input.report.reference_index_complete;
    normalize_refs(&mut evidence.retained_refs);
    normalize_refs(&mut evidence.remote_refs);
    normalize_refs(&mut evidence.evidence_refs);
    normalize_refs(&mut evidence.reference_index_refs);
    normalize_refs(&mut evidence.remote_gc_refs);
    let plan = store_gc_plan(GcPlanInput {
        root: input.retention_root,
        subsystem: WORLD_DISTRIBUTION_SUBSYSTEM,
        object_ref: input.object_ref,
        object_kind: input.object_kind,
        retention_class: input.retention_class,
        action: input.action,
        evidence: &evidence,
    })?;
    Ok(WorldRetentionHandoff {
        report,
        plan,
        report_granted_deletion_authority: false,
    })
}

fn normalize_refs(references: &mut Vec<String>) {
    references.sort();
    references.dedup();
}
