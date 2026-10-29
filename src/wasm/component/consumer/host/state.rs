//! Bounded readback shared only by the owning live or replay host.

const PUBLICATIONS_PER_EFFECT: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedEffect {
    pub record: crate::system_extension::NativeCallbackValue,
    pub response_bytes: Vec<u8>,
    pub entry: crate::deterministic_replay::EffectLogEntry,
}

/// A live Session publication, including attempts without an accepted receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Publication {
    pub request_ref: String,
    pub value_ref: String,
    pub byte_count: u64,
    pub receipt: Option<crate::system_extension::ValuePublicationReceipt>,
    pub failure: Option<crate::system_extension::ValuePortFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Records {
    pub effects: Vec<RecordedEffect>,
    pub publications: Vec<Publication>,
    pub failure: Option<super::super::super::Denial>,
    /// `None` for live invocation, exact number still unconsumed for replay.
    pub replay_remaining: Option<usize>,
}

impl Records {
    pub fn has_uncertain_publication(&self) -> bool {
        self.publications
            .iter()
            .any(|attempt| attempt.failure.as_ref().is_some_and(|failure| failure.may_have_published()))
    }
}

/// Readback survives the engine taking ownership of the invocation host.
#[derive(Debug, Clone)]
pub struct Recording {
    pub(super) records: std::sync::Arc<std::sync::Mutex<Records>>,
    maximum_effects: usize,
    pub(super) maximum_publications: usize,
}

impl Recording {
    pub fn empty() -> Self {
        Self::with_capacity(0, None)
    }

    pub(super) fn with_capacity(maximum_effects: usize, replay_remaining: Option<usize>) -> Self {
        let maximum_publications = if replay_remaining.is_none() {
            maximum_effects * PUBLICATIONS_PER_EFFECT
        } else {
            0
        };
        Self {
            records: std::sync::Arc::new(std::sync::Mutex::new(Records {
                effects: Vec::with_capacity(maximum_effects),
                publications: Vec::with_capacity(maximum_publications),
                failure: None,
                replay_remaining,
            })),
            maximum_effects,
            maximum_publications,
        }
    }

    pub fn snapshot(&self) -> super::super::super::Result<Records> {
        self.records.lock().map(|records| records.clone()).map_err(|_| super::poisoned_recording())
    }

    pub fn ensure_replay_complete(&self) -> super::super::super::Result<()> {
        let records = self.records.lock().map_err(|_| super::poisoned_recording())?;
        if let Some(denial) = &records.failure {
            return Err(denial.clone());
        }
        match records.replay_remaining {
            Some(0) => Ok(()),
            Some(_) => {
                Err(super::super::super::Denial::new("component replay left unconsumed recorded host boundaries"))
            }
            None => Err(super::super::super::Denial::new("component invocation is live, not offline replay")),
        }
    }

    pub(super) fn admit_boundary(&self) -> super::super::super::Result<()> {
        let records = self.records.lock().map_err(|_| super::poisoned_recording())?;
        if let Some(denial) = &records.failure {
            return Err(denial.clone());
        }
        if records.effects.len() >= self.maximum_effects {
            return Err(super::super::super::Denial::new(
                "component host recording exhausted its admitted boundary capacity",
            ));
        }
        Ok(())
    }

    pub(super) fn retain(&self, effect: RecordedEffect) -> super::super::super::Result<()> {
        let mut records = self.records.lock().map_err(|_| super::poisoned_recording())?;
        if records.effects.len() >= self.maximum_effects {
            return Err(super::super::super::Denial::new(
                "component host completed an effect without its reserved recording slot; reconciliation required",
            ));
        }
        if let Some(remaining) = &mut records.replay_remaining {
            *remaining = remaining
                .checked_sub(1)
                .ok_or_else(|| super::super::super::Denial::new("component replay consumed beyond its recording"))?;
        }
        records.effects.push(effect);
        Ok(())
    }

    pub(super) fn fail(&self, denial: &super::super::super::Denial) {
        if let Ok(mut records) = self.records.lock()
            && records.failure.is_none()
        {
            records.failure = Some(denial.clone());
        }
    }
}
