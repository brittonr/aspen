impl super::Database<'_> {
    pub fn retained(
        &self,
        branch: &molten_core::world_head::WorldBranchId,
    ) -> crate::error::Result<molten_core::world_witness::RetainedWitnessState> {
        let read = redb::ReadableDatabase::begin_read(self.heads.database()).map_err(super::failure)?;
        let highest = read.open_table(super::HIGHEST).map_err(super::failure)?;
        if let Some(value) = redb::ReadableTable::get(&highest, branch.as_str()).map_err(super::failure)? {
            return Ok(molten_core::world_witness::RetainedWitnessState::Present(
                super::super::records::parse_highest(value.value())?,
            ));
        }
        let heads = read.open_table(super::HEADS).map_err(super::failure)?;
        Ok(if redb::ReadableTable::get(&heads, branch.as_str()).map_err(super::failure)?.is_some() {
            molten_core::world_witness::RetainedWitnessState::Missing
        } else {
            molten_core::world_witness::RetainedWitnessState::Empty
        })
    }

    pub fn retained_local(
        &self,
        branch: &molten_core::world_head::WorldBranchId,
    ) -> crate::error::Result<molten_core::world_witness::RetainedWitnessState> {
        let retained = self.retained(branch)?;
        if !matches!(retained, molten_core::world_witness::RetainedWitnessState::Missing) {
            return Ok(retained);
        }
        let read = redb::ReadableDatabase::begin_read(self.heads.database()).map_err(super::failure)?;
        let required = read.open_table(super::REQUIRED).map_err(super::failure)?;
        Ok(if redb::ReadableTable::get(&required, branch.as_str()).map_err(super::failure)?.is_some() {
            molten_core::world_witness::RetainedWitnessState::Missing
        } else {
            molten_core::world_witness::RetainedWitnessState::Empty
        })
    }
    pub fn save_reservation(&mut self, operation: &str, bytes: &[u8]) -> crate::error::Result<()> {
        let write = self.heads.database().begin_write().map_err(super::failure)?;
        write
            .open_table(super::RESERVATIONS)
            .map_err(super::failure)?
            .insert(operation, bytes)
            .map_err(super::failure)?;
        write.commit().map_err(super::failure)
    }

    pub fn reservation(&self, operation: &str) -> crate::error::Result<Option<Vec<u8>>> {
        let read = redb::ReadableDatabase::begin_read(self.heads.database()).map_err(super::failure)?;
        let table = read.open_table(super::RESERVATIONS).map_err(super::failure)?;
        Ok(redb::ReadableTable::get(&table, operation)
            .map_err(super::failure)?
            .map(|value| value.value().to_vec()))
    }

    pub fn current(
        &self,
        branch: &molten_core::world_head::WorldBranchId,
    ) -> crate::error::Result<Option<molten_core::world_head::WorldHeadState>> {
        let read = redb::ReadableDatabase::begin_read(self.heads.database()).map_err(super::failure)?;
        let table = read.open_table(super::HEADS).map_err(super::failure)?;
        redb::ReadableTable::get(&table, branch.as_str())
            .map_err(super::failure)?
            .map(|value| crate::world_head::parse_canonical_world_head_state(value.value()))
            .transpose()
    }

    pub fn reconciliation(&self, operation: &str) -> crate::error::Result<Option<Vec<u8>>> {
        let read = redb::ReadableDatabase::begin_read(self.heads.database()).map_err(super::failure)?;
        let table = read.open_table(super::RECONCILIATION).map_err(super::failure)?;
        Ok(redb::ReadableTable::get(&table, operation)
            .map_err(super::failure)?
            .map(|value| value.value().to_vec()))
    }

    pub fn save_reconciliation(&mut self, operation: &str, bytes: &[u8]) -> crate::error::Result<()> {
        let write = self.heads.database().begin_write().map_err(super::failure)?;
        write
            .open_table(super::RECONCILIATION)
            .map_err(super::failure)?
            .insert(operation, bytes)
            .map_err(super::failure)?;
        write.commit().map_err(super::failure)
    }

    pub fn stage_bytes(&self, operation: &str) -> crate::error::Result<Option<Vec<u8>>> {
        let read = redb::ReadableDatabase::begin_read(self.heads.database()).map_err(super::failure)?;
        let table = read.open_table(super::STAGES).map_err(super::failure)?;
        Ok(redb::ReadableTable::get(&table, operation)
            .map_err(super::failure)?
            .map(|value| value.value().to_vec()))
    }

    pub fn operation_receipt(&self, operation: &str) -> crate::error::Result<Option<super::super::records::Receipt>> {
        let read = redb::ReadableDatabase::begin_read(self.heads.database()).map_err(super::failure)?;
        let table = read.open_table(super::WORLD_WITNESS_RECEIPTS).map_err(super::failure)?;
        redb::ReadableTable::get(&table, operation)
            .map_err(super::failure)?
            .map(|value| super::super::records::receipt::parse(value.value()))
            .transpose()
    }
}

pub(super) fn head(
    write: &redb::WriteTransaction,
    branch: &str,
) -> crate::error::Result<Option<molten_core::world_head::WorldHeadState>> {
    let table = write.open_table(super::HEADS).map_err(super::failure)?;
    redb::ReadableTable::get(&table, branch)
        .map_err(super::failure)?
        .map(|value| crate::world_head::parse_canonical_world_head_state(value.value()))
        .transpose()
}

pub(super) fn retained_in_transaction(
    write: &redb::WriteTransaction,
    branch: &str,
    has_head: bool,
) -> crate::error::Result<molten_core::world_witness::RetainedWitnessState> {
    let table = write.open_table(super::HIGHEST).map_err(super::failure)?;
    Ok(match redb::ReadableTable::get(&table, branch).map_err(super::failure)? {
        Some(value) => molten_core::world_witness::RetainedWitnessState::Present(super::super::records::parse_highest(
            value.value(),
        )?),
        None if has_head => molten_core::world_witness::RetainedWitnessState::Missing,
        None => molten_core::world_witness::RetainedWitnessState::Empty,
    })
}
