pub mod contract;
pub mod inclusion;
pub mod normalize;
pub mod process;
mod projection;
mod recovery;
mod types;

pub use types::Error;
pub use types::ErrorKind;
pub use types::Observation;
pub use types::Reservation;

/// The caller supplies the transport capability; no endpoint or trust discovery occurs.
pub trait Transport {
    fn exchange(
        &mut self,
        request: &release_channel_index::model::SignedRequest,
    ) -> Result<release_channel_index::model::SignedReply, String>;
}

/// Shell-selected request entropy and observation clock; the provider never reads ambient
/// randomness or time.
pub trait Ports: crate::fabric_time::CryptographicEntropySource + super::WallClock {}

impl<P: crate::fabric_time::CryptographicEntropySource + super::WallClock> Ports for P {}

pub struct Trust {
    pub key: [u8; 32],
    pub contract: molten_core::world_witness::ProviderContract,
}

impl Trust {
    fn validate(&self) -> Result<(), Error> {
        if self.contract.provider_id != identity(&self.key)
            || self.contract.contract_id != "release-channel-index-v1"
            || self.contract.license != "MIT"
            || self.contract.source_revision.len() != 40
            || !self
                .contract
                .source_revision
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(Error::invalid("unsupported independently installed provider contract"));
        }
        Ok(())
    }
}

pub fn identity(key: &[u8; 32]) -> String {
    format!("ed25519:{}", projection::hex(key))
}

pub struct Index<T, P> {
    namespace: release_channel_index::model::Namespace,
    provider_key: [u8; 32],
    contract: molten_core::world_witness::ProviderContract,
    writer: ed25519_dalek::SigningKey,
    transport: T,
    ports: P,
}

impl<T: Transport, P: Ports> Index<T, P> {
    pub fn new(
        namespace: release_channel_index::model::Namespace,
        trust: Trust,
        writer: ed25519_dalek::SigningKey,
        transport: T,
        ports: P,
    ) -> Result<Self, Error> {
        namespace.validate().map_err(Error::invalid)?;
        trust.validate()?;
        Ok(Self {
            namespace,
            provider_key: trust.key,
            contract: trust.contract,
            writer,
            transport,
            ports,
        })
    }

    /// Observation time comes from the shell-selected clock port, never the host clock.
    pub fn observed_at(&mut self) -> crate::error::Result<u64> {
        super::WallClock::unix_seconds(&mut self.ports)
    }

    pub fn query(&mut self, minimum_revision: u64) -> Result<Observation, Error> {
        let request = self.request(release_channel_index::model::Operation::Query)?;
        self.exchange(&request, minimum_revision, None)
    }

    pub fn observe_current(
        &mut self,
        claim: &release_channel_index::model::Claim,
        minimum_revision: u64,
    ) -> Result<Observation, Error> {
        let observation = self.query(minimum_revision)?;
        release_channel_index::client::require_current(&observation.snapshot, claim).map_err(Error::invalid)?;
        Ok(observation)
    }

    /// Query before reserve; an exact pending reservation is recovered, never blindly duplicated.
    pub fn reserve(
        &mut self,
        expected_revision: u64,
        claim: release_channel_index::model::Claim,
    ) -> Result<Reservation, Error> {
        let before = self.query(expected_revision)?;
        if let Some(pending) = &before.snapshot.pending {
            if pending.claim == claim && pending.status == release_channel_index::model::PendingStatus::Reserved {
                return Ok(self.reservation(claim, pending.reservation, before.snapshot.revision));
            }
            return Err(Error::invalid("external reservation conflict"));
        }
        if before.snapshot.revision != expected_revision {
            return Err(Error::invalid("external revision changed before reserve"));
        }
        let operation = release_channel_index::model::Operation::Reserve {
            expected_revision,
            claim: claim.clone(),
        };
        let request = self.request(operation)?;
        let id = release_channel_index::crypto::reservation(&request).map_err(Error::invalid)?;
        let revision = expected_revision.checked_add(1).ok_or_else(|| Error::invalid("revision overflow"))?;
        let reservation = self.reservation(claim, id, revision);
        let after = self.exchange(&request, revision, Some(&reservation))?;
        self.require_reserved(&reservation, &after)?;
        Ok(reservation)
    }

    /// Fresh authenticated pending facts fence the consumer's local final-CAS boundary.
    pub fn observe_reserved(&mut self, reservation: &Reservation) -> Result<Observation, Error> {
        self.bind(reservation)?;
        let observation = self.query(reservation.revision)?;
        self.require_reserved(reservation, &observation)?;
        Ok(observation)
    }

    fn require_reserved(&self, reservation: &Reservation, observation: &Observation) -> Result<(), Error> {
        let snapshot = &observation.snapshot;
        let is_exact = snapshot.pending.as_ref().is_some_and(|pending| {
            pending.reservation == reservation.id
                && pending.claim == reservation.claim
                && pending.status == release_channel_index::model::PendingStatus::Reserved
        });
        if !is_exact || snapshot.highest.as_ref() != Some(&reservation.claim) {
            return Err(Error::invalid("external pending claim is not the exact reserved claim"));
        }
        Ok(())
    }

    fn reservation(&self, claim: release_channel_index::model::Claim, id: [u8; 32], revision: u64) -> Reservation {
        Reservation {
            namespace: self.namespace.clone(),
            provider_key: self.provider_key,
            claim,
            id,
            revision,
        }
    }

    fn bind(&self, reservation: &Reservation) -> Result<(), Error> {
        if reservation.namespace != self.namespace || reservation.provider_key != self.provider_key {
            return Err(Error::invalid("reservation provider or namespace substitution"));
        }
        Ok(())
    }

    fn request(
        &mut self,
        operation: release_channel_index::model::Operation,
    ) -> Result<release_channel_index::model::SignedRequest, Error> {
        let mut challenge = [0; 32];
        crate::fabric_time::CryptographicEntropySource::fill_secret(&mut self.ports, &mut challenge)
            .map_err(|error| Error::invalid(error.to_string()))?;
        release_channel_index::crypto::authenticate(&self.writer, release_channel_index::model::Request {
            namespace: self.namespace.clone(),
            challenge,
            operation,
        })
        .map_err(Error::invalid)
    }

    fn exchange(
        &mut self,
        request: &release_channel_index::model::SignedRequest,
        minimum_revision: u64,
        reservation: Option<&Reservation>,
    ) -> Result<Observation, Error> {
        let reply = self.transport.exchange(request).map_err(|detail| Error {
            kind: if reservation.is_some() {
                ErrorKind::Uncertain
            } else {
                ErrorKind::Unavailable
            },
            detail,
            reservation: reservation.cloned().map(Box::new),
        })?;
        let snapshot =
            release_channel_index::client::verify_reply(&self.provider_key, &request.request, &reply, minimum_revision)
                .map_err(|detail| Error {
                    kind: if reservation.is_some() {
                        ErrorKind::Uncertain
                    } else {
                        ErrorKind::Invalid
                    },
                    detail,
                    reservation: reservation.cloned().map(Box::new),
                })?;
        Ok(Observation {
            snapshot,
            reply,
            request: request.request.clone(),
        })
    }
}

#[cfg(test)]
mod tests;
