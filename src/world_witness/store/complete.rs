impl super::Database<'_> {
    pub(crate) fn complete(
        &mut self,
        stage: &super::super::Stage,
        receipt: &super::super::records::Receipt,
    ) -> crate::error::Result<()> {
        stage.validate()?;
        super::super::records::receipt::validate(receipt)?;
        if receipt.operation_id != stage.operation_ref
            || receipt.witness_currentness != super::super::records::RoleDecision::Admitted
        {
            return Err(super::failure("external completion does not bind operation"));
        }
        let highest = receipt.highest.as_ref().ok_or_else(|| super::failure("completion lacks highest state"))?;
        let write = self.heads.database().begin_write().map_err(super::failure)?;
        let branch = stage.staged.transition.after.branch_id.as_str();
        if super::read::head(&write, branch)?.as_ref() != Some(&stage.staged.transition.after) {
            return Err(super::failure("local head changed before external completion persistence"));
        }
        let retained = super::read::retained_in_transaction(&write, branch, true)?;
        let molten_core::world_witness::RetainedWitnessState::Present(previous) = retained else {
            return Err(super::failure("completion cannot reset missing highest state"));
        };
        if previous.subject != highest.subject
            || previous.checkpoint.provider_id != highest.checkpoint.provider_id
            || previous.checkpoint.revision > highest.checkpoint.revision
        {
            return Err(super::failure("completion substitutes or regresses retained witness state"));
        }
        let bytes = super::super::records::canonical_highest(highest)?;
        write
            .open_table(super::HIGHEST)
            .map_err(super::failure)?
            .insert(branch, bytes.as_slice())
            .map_err(super::failure)?;
        let bytes = super::super::records::receipt::canonical(receipt)?;
        let reference = super::super::records::receipt::identity(receipt)?;
        write
            .open_table(super::TRANSITIONS)
            .map_err(super::failure)?
            .insert(reference.as_str(), bytes.as_slice())
            .map_err(super::failure)?;
        write
            .open_table(super::WORLD_WITNESS_RECEIPTS)
            .map_err(super::failure)?
            .insert(stage.operation_ref.as_str(), bytes.as_slice())
            .map_err(super::failure)?;
        write.commit().map_err(super::failure)
    }
}
