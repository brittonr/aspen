use molten_core::dag_sync::DagSyncDecision;
use molten_core::world_distribution::*;

use super::CanonicalWorldDistributionRecord;
use super::canonical_world_sync_receipt;
use crate::dag_sync::DagAuthorityPort;
use crate::dag_sync::DagContentVerificationPort;
use crate::dag_sync::DagObservationPort;
use crate::dag_sync::DagProgressPort;
use crate::dag_sync::DagReceiptPort;
use crate::dag_sync::DagResourcePort;
use crate::dag_sync::DagSyncOutcome;
use crate::dag_sync::DagSyncPorts;
use crate::dag_sync::DagTransportPort;
use crate::dag_sync::run_dag_sync;
use crate::error::MoltenError;
use crate::error::Result;

pub trait WorldDistributionReceiptPort {
    fn publish_world_distribution_receipt(&mut self, receipt: &CanonicalWorldDistributionRecord) -> Result<()>;
}

pub struct WorldSyncPorts<'a, A, R, T, C, P, O, E, W> {
    pub dag: DagSyncPorts<'a, A, R, T, C, P, O, E>,
    pub receipts: &'a mut W,
}

#[derive(Debug, Clone)]
pub struct WorldSyncOutcome {
    pub initial_plan: WorldClosurePlan,
    pub dag: DagSyncOutcome,
    pub complete: bool,
    pub missing: Vec<WorldObjectRef>,
    pub activation_authorized: bool,
    pub canonical_receipt: CanonicalWorldDistributionRecord,
}

// r[impl molten.world_distribution.closure]
// r[impl molten.world_distribution.partial]
pub fn run_world_sync<A, R, T, C, P, O, E, W>(
    projection: &WorldDagProjection,
    context: &WorldSyncContext,
    ports: WorldSyncPorts<'_, A, R, T, C, P, O, E, W>,
) -> Result<WorldSyncOutcome>
where
    A: DagAuthorityPort,
    R: DagResourcePort,
    T: DagTransportPort,
    C: DagContentVerificationPort,
    P: DagProgressPort,
    O: DagObservationPort,
    E: DagReceiptPort,
    W: WorldDistributionReceiptPort,
{
    let initial_plan = plan_world_closure(projection, context)
        .map_err(|issues| MoltenError::invalid_harness(format!("world closure planning denied: {issues:?}")))?;
    let dag = run_dag_sync(&projection.graph, initial_plan.request.clone(), ports.dag)?;
    if dag.plan.plan_ref != initial_plan.shared_plan.plan_ref {
        return Err(MoltenError::invalid_harness("world closure plan drifted before DAG synchronization"));
    }
    let missing = dag
        .receipt
        .missing
        .iter()
        .map(|object| {
            dag_object_to_world(object, &projection.objects)
                .ok_or_else(|| MoltenError::invalid_harness("DAG receipt contains an untyped world object"))
        })
        .collect::<Result<Vec<_>>>()?;
    let complete = dag.receipt.decision == DagSyncDecision::Complete && missing.is_empty();
    let canonical_receipt =
        canonical_world_sync_receipt(&initial_plan, &dag.canonical_receipt.record_ref, complete, dag.receipt.verified)?;
    ports.receipts.publish_world_distribution_receipt(&canonical_receipt)?;
    Ok(WorldSyncOutcome {
        initial_plan,
        dag,
        complete,
        missing,
        activation_authorized: false,
        canonical_receipt,
    })
}
