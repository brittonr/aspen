/// Complementary transparency evidence. It deliberately has no current-head field.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Evidence {
    evidence: release_channel_witness::types::Evidence,
    consistency_predecessor: Option<release_channel_witness::types::Checkpoint>,
}

pub fn append<P: release_channel_witness::provider::Provider>(
    provider: &mut P,
    statement: &release_channel_witness::types::Statement,
    previous: Option<&release_channel_witness::types::Checkpoint>,
) -> Result<Evidence, release_channel_witness::types::Error> {
    let submitted = provider.append(statement)?;
    verify(provider, statement, &submitted.proof, previous)
}

pub fn verify<P: release_channel_witness::provider::Provider>(
    provider: &mut P,
    statement: &release_channel_witness::types::Statement,
    proof: &str,
    previous: Option<&release_channel_witness::types::Checkpoint>,
) -> Result<Evidence, release_channel_witness::types::Error> {
    let evidence = provider.verify(statement, proof)?;
    if evidence.statement != *statement {
        return Err(release_channel_witness::types::Error::Invalid(
            "complementary inclusion statement substitution".into(),
        ));
    }
    if let Some(previous) = previous {
        provider.consistency(previous, &evidence.checkpoint)?;
    }
    Ok(Evidence {
        evidence,
        consistency_predecessor: previous.cloned(),
    })
}

impl Evidence {
    /// Inclusion DTO only: callers still need independently authenticated index currentness.
    pub fn normalize(
        &self,
        subject: molten_core::world_witness::WitnessSubject,
    ) -> Result<molten_core::world_witness::InclusionObservation, release_channel_witness::types::Error> {
        let digest = super::normalize::claim_digest(&subject.claim_ref)
            .map_err(|error| release_channel_witness::types::Error::Invalid(error.to_string()))?;
        if digest != self.evidence.statement.metadata_hash {
            return Err(release_channel_witness::types::Error::Invalid(
                "complementary inclusion claim substitution".into(),
            ));
        }
        Ok(molten_core::world_witness::InclusionObservation {
            subject,
            checkpoint_id: format!(
                "sigsum:{}:{}:{}",
                self.evidence.checkpoint.log, self.evidence.checkpoint.size, self.evidence.checkpoint.root
            ),
            verified: true,
        })
    }
}
