//! Diagnostic data and a client for the retained worker; neither carries mutation authority.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    pub read: crate::collaboration_scope::RequestRef,
    pub state: crate::collaboration_scope::StateRef,
    pub currentness: crate::collaboration_scope::Currentness,
    pub unix_seconds: u64,
}

impl Observation {
    pub fn value(&self) -> preserves::IOValue {
        crate::preserves_rail::record("collaboration-scope-observation-v1", vec![
            crate::preserves_rail::string(self.read.as_str()),
            crate::preserves_rail::string(self.state.as_str()),
            crate::collaboration_scope::currentness_value(&self.currentness),
            crate::preserves_rail::u64_value(self.unix_seconds),
        ])
    }

    /// Parsing diagnostic data never constructs a `CurrentState` authority marker.
    pub fn parse(value: &preserves::IOValue) -> crate::error::Result<Self> {
        let f = crate::collaboration_scope::decode::fields(value, "collaboration-scope-observation-v1", 4)?;
        Ok(Self {
            read: crate::collaboration_scope::RequestRef::new(crate::collaboration_scope::decode::string(&f[0])?)
                .map_err(|_| crate::collaboration_scope::denied(crate::collaboration_scope::Denial::Domain))?,
            state: crate::collaboration_scope::StateRef::new(crate::collaboration_scope::decode::string(&f[1])?)
                .map_err(|_| crate::collaboration_scope::denied(crate::collaboration_scope::Denial::Domain))?,
            currentness: crate::collaboration_scope::parse_currentness(&f[2])?,
            unix_seconds: crate::collaboration_scope::decode::integer(&f[3])?,
        })
    }
}

/// A diagnostic-only channel to the worker that owns the live replica and ledger.
pub struct Observer {
    pub(super) sender: tokio::sync::mpsc::Sender<super::Message>,
    pub(super) timeout: std::time::Duration,
}

impl Observer {
    pub fn observe(&self, read: crate::collaboration_scope::RequestRef) -> crate::error::Result<Observation> {
        let (reply, result) = std::sync::mpsc::sync_channel(1);
        self.sender
            .try_send(super::Message::Observe(read, reply))
            .map_err(|_| crate::collaboration_scope::denied(crate::collaboration_scope::Denial::Uncertain))?;
        result
            .recv_timeout(self.timeout + std::time::Duration::from_secs(1))
            .map_err(|_| crate::collaboration_scope::denied(crate::collaboration_scope::Denial::Uncertain))?
            .map_err(crate::error::Failure::invalid_harness)
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn codec_preserves_signed_inputs_and_rejects_malformed_fields() {
        let observation = super::Observation {
            read: crate::collaboration_scope::RequestRef::new(crate::collaboration_scope::fixtures::reference(
                "diagnostic-read",
            ))
            .expect("read"),
            state: crate::collaboration_scope::StateRef::new(crate::collaboration_scope::fixtures::reference(
                "diagnostic-state",
            ))
            .expect("state"),
            currentness: crate::collaboration_scope::Currentness {
                service_generation: 7,
                consistency_epoch: 11,
                committed_index: 19,
                logical_now: 19,
                revocation: molten_core::nominal::RevocationRef::new(crate::collaboration_scope::fixtures::reference(
                    "trust",
                ))
                .expect("trust"),
                observation: molten_core::nominal::EvidenceRef::new(crate::collaboration_scope::fixtures::reference(
                    "read-receipt",
                ))
                .expect("evidence"),
            },
            unix_seconds: u64::MAX,
        };
        let encoded = crate::preserves_rail::canonical_bytes(&observation.value()).expect("encode");
        let value = crate::preserves_rail::parse_canonical_bytes(&encoded).expect("canonical data");
        assert_eq!(super::Observation::parse(&value).expect("decode"), observation);
        let fields = crate::collaboration_scope::decode::fields(&value, "collaboration-scope-observation-v1", 4)
            .expect("fields");
        for (index, replacement) in [
            (0, crate::preserves_rail::string("not-a-reference")),
            (1, crate::preserves_rail::string("not-a-reference")),
            (2, crate::preserves_rail::string("not-currentness")),
            (3, crate::preserves_rail::string("not-unix-seconds")),
        ] {
            let mut malformed = fields.clone();
            malformed[index] = replacement;
            assert!(
                super::Observation::parse(&crate::preserves_rail::record(
                    "collaboration-scope-observation-v1",
                    malformed,
                ))
                .is_err()
            );
        }
        assert!(
            super::Observation::parse(&crate::preserves_rail::record(
                "collaboration-scope-observation-v1",
                fields[..3].to_vec(),
            ))
            .is_err()
        );
    }
}
