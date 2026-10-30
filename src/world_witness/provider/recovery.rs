impl<T: super::Transport, P: super::Ports> super::Index<T, P> {
    /// Observe an uncertain append without dispatching a new mutation.
    pub fn recover_reservation(
        &mut self,
        claim: &release_channel_index::model::Claim,
        minimum_revision: u64,
    ) -> Result<Option<super::types::Reservation>, super::types::Error> {
        let observation = self.query(minimum_revision)?;
        if let Some(pending) = &observation.snapshot.pending {
            if pending.claim == *claim && pending.status == release_channel_index::model::PendingStatus::Reserved {
                return Ok(Some(self.reservation(claim.clone(), pending.reservation, observation.snapshot.revision)));
            }
            return Err(super::types::Error::invalid("uncertain append has conflicting or orphan pending state"));
        }
        if observation.snapshot.highest.as_ref() == Some(claim) {
            return Err(super::types::Error::invalid(
                "matching highest exists without recoverable pending reservation",
            ));
        }
        Ok(None)
    }

    /// Call only after the consumer has proved its exact local CAS committed.
    pub fn commit(
        &mut self,
        reservation: &super::types::Reservation,
    ) -> Result<super::types::Observation, super::types::Error> {
        self.bind(reservation)?;
        let before = self.query(reservation.revision)?;
        if release_channel_index::client::require_current(&before.snapshot, &reservation.claim).is_ok() {
            return Ok(before);
        }
        self.require_reserved(reservation, &before)?;
        let operation = release_channel_index::model::Operation::Commit {
            expected_revision: before.snapshot.revision,
            reservation: reservation.id,
            expected_predecessor: reservation.claim.predecessor,
        };
        let request = self.request(operation)?;
        let after = self.exchange(&request, before.snapshot.revision, Some(reservation))?;
        release_channel_index::client::require_current(&after.snapshot, &reservation.claim)
            .map_err(super::types::Error::invalid)?;
        Ok(after)
    }

    /// Call only after the consumer has proved the staged local CAS did not commit.
    pub fn orphan(
        &mut self,
        reservation: &super::types::Reservation,
    ) -> Result<super::types::Observation, super::types::Error> {
        self.change_pending(reservation, false)
    }

    /// Explicit operator/reconciliation action; never erases the retained highest claim.
    pub fn resolve_orphan(
        &mut self,
        reservation: &super::types::Reservation,
    ) -> Result<super::types::Observation, super::types::Error> {
        self.change_pending(reservation, true)
    }

    fn change_pending(
        &mut self,
        reservation: &super::types::Reservation,
        resolve: bool,
    ) -> Result<super::types::Observation, super::types::Error> {
        self.bind(reservation)?;
        let before = self.query(reservation.revision)?;
        let pending = before
            .snapshot
            .pending
            .as_ref()
            .ok_or_else(|| super::types::Error::invalid("missing pending reservation"))?;
        if pending.reservation != reservation.id || pending.claim != reservation.claim {
            return Err(super::types::Error::invalid("pending reservation conflict"));
        }
        if !resolve && pending.status == release_channel_index::model::PendingStatus::Orphan {
            return Ok(before);
        }
        if resolve && pending.status != release_channel_index::model::PendingStatus::Orphan {
            return Err(super::types::Error::invalid("explicit orphan required before resolution"));
        }
        let expected_revision = before.snapshot.revision;
        let operation = if resolve {
            release_channel_index::model::Operation::ResolveOrphan {
                expected_revision,
                reservation: reservation.id,
            }
        } else {
            release_channel_index::model::Operation::Orphan {
                expected_revision,
                reservation: reservation.id,
            }
        };
        let request = self.request(operation)?;
        let after = self.exchange(&request, expected_revision, Some(reservation))?;
        let is_valid_pending = if resolve {
            after.snapshot.pending.is_none()
        } else {
            after.snapshot.pending.as_ref().is_some_and(|pending| {
                pending.reservation == reservation.id
                    && pending.claim == reservation.claim
                    && pending.status == release_channel_index::model::PendingStatus::Orphan
            })
        };
        if !is_valid_pending
            || after.snapshot.highest != before.snapshot.highest
            || after.snapshot.committed != before.snapshot.committed
        {
            return Err(super::types::Error::invalid("orphan operation changed committed or retained highest state"));
        }
        Ok(after)
    }
}
