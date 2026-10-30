mod complete;
mod finalize;
mod read;

pub(crate) const HIGHEST: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("world_witness_highest_v1");
const STAGES: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("world_witness_stages_v1");
const CLAIMS: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("world_witness_claims_v1");
const RESERVATIONS: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("world_witness_reservations_v1");
const HEADS: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("world_heads_v1");
const RECONCILIATION: redb::TableDefinition<&str, &[u8]> =
    redb::TableDefinition::new("world_witness_reconciliation_v1");
const TRANSITIONS: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("world_head_transitions_v1");
const WORLD_WITNESS_RECEIPTS: redb::TableDefinition<&str, &[u8]> =
    redb::TableDefinition::new("world_witness_receipts_v1");
pub(crate) const REQUIRED: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("world_witness_required_v1");

pub struct Database<'a> {
    heads: &'a mut crate::world_head::LocalWorldHeadStore,
}

impl<'a> Database<'a> {
    pub fn new(heads: &'a mut crate::world_head::LocalWorldHeadStore) -> crate::error::Result<Self> {
        let write = heads.database().begin_write().map_err(failure)?;
        for definition in [
            HIGHEST,
            STAGES,
            CLAIMS,
            RESERVATIONS,
            RECONCILIATION,
            REQUIRED,
            WORLD_WITNESS_RECEIPTS,
        ] {
            write.open_table(definition).map_err(failure)?;
        }
        write.commit().map_err(failure)?;
        Ok(Self { heads })
    }

    // r[impl molten.world_witness.finalization]
    pub fn stage(&mut self, stage: &super::Stage) -> crate::error::Result<crate::world_head::WorldHeadMutationOutcome> {
        stage.validate()?;
        let write = self.heads.database().begin_write().map_err(failure)?;
        if read::head(&write, stage.staged.transition.after.branch_id.as_str())? != stage.staged.transition.before {
            return Ok(crate::world_head::WorldHeadMutationOutcome::Stale);
        }
        {
            let mut stages = write.open_table(STAGES).map_err(failure)?;
            if let Some(existing) = redb::ReadableTable::get(&stages, stage.operation_ref.as_str()).map_err(failure)?
                && existing.value() != stage.bytes
            {
                return Err(failure("operation identity has different staged bytes"));
            }
            stages.insert(stage.operation_ref.as_str(), stage.bytes.as_slice()).map_err(failure)?;
            let mut claims = write.open_table(CLAIMS).map_err(failure)?;
            claims.insert(stage.operation_ref.as_str(), stage.claim_bytes.as_slice()).map_err(failure)?;
            let binding = super::recovery::binding::prepare(stage)?;
            let bytes = super::recovery::binding::canonical(&binding)?;
            write
                .open_table(RECONCILIATION)
                .map_err(failure)?
                .insert(stage.operation_ref.as_str(), bytes.as_slice())
                .map_err(failure)?;
            if stage.staged.selected_profile == molten_core::world_witness::AssuranceProfile::IndependentWitnessV1 {
                write
                    .open_table(REQUIRED)
                    .map_err(failure)?
                    .insert(stage.staged.transition.after.branch_id.as_str(), stage.operation_ref.as_bytes())
                    .map_err(failure)?;
            }
        }
        Ok(match write.commit() {
            Ok(()) => crate::world_head::WorldHeadMutationOutcome::Applied,
            Err(_) => crate::world_head::WorldHeadMutationOutcome::Uncertain,
        })
    }
}

pub struct FinalizeInput<'a> {
    pub stage: &'a super::Stage,
    pub claim: &'a crate::world_head::CanonicalWorldHeadClaim,
}

pub(crate) fn failure(error: impl std::fmt::Display) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(format!("world witness store: {error}"))
}
