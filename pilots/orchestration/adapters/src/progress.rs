// SPDX-License-Identifier: AGPL-3.0-or-later
//! Redb reconstruction DTO and bridge to the single authoritative committed-stream core.
use molten_batch_application::Error;
use molten_batch_core::*;
use molten_orchestrator_codec as codec;
use serde::Deserialize;
use serde::Serialize;

use crate::committed_stream as stream;

pub(crate) const STORAGE_SCHEMA: &str = "controller-progress.v1";
pub(crate) const OWNER_SCHEMA: &str = "controller-progress-owner.v1";

/// One small durable binding, not a second event ledger. The reconstruction
/// digest checks raw cache bytes; semantic identity remains canonical Preserves.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct JournalOwner {
    pub state_ref: Id,
    pub projected_state_ref: Id,
    pub journal_ref: Id,
    pub reconstruction_digest: Id,
}
impl JournalOwner {
    pub(crate) fn new(state_ref: Id, journal: &codec::Encoded) -> Self {
        Self {
            state_ref,
            projected_state_ref: state_ref,
            journal_ref: journal.id,
            reconstruction_digest: codec::digest(&journal.projection),
        }
    }
    pub(crate) fn validate(&self, state_ref: Id, canonical: &[u8], reconstruction: &[u8]) -> Result<(), Error> {
        if self.state_ref != state_ref
            || self.journal_ref != codec::digest(canonical)
            || self.reconstruction_digest != codec::digest(reconstruction)
        {
            return Err(Error::Invalid("progress owner/state/journal binding mismatch".into()));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Journal {
    service_id: String,
    epoch: u64,
    revision: u64,
    first_retained_revision: u64,
    state_ref: Option<Id>,
    events: Vec<ProgressEvent>,
}
impl From<&ProgressCursor> for stream::StreamCursor {
    fn from(c: &ProgressCursor) -> Self {
        Self {
            service_id: c.service_id.clone(),
            stream_epoch: c.stream_epoch,
            revision: c.revision,
            ordinal: c.ordinal,
        }
    }
}
impl From<stream::StreamCursor> for ProgressCursor {
    fn from(c: stream::StreamCursor) -> Self {
        Self {
            service_id: c.service_id,
            stream_epoch: c.stream_epoch,
            revision: c.revision,
            ordinal: c.ordinal,
        }
    }
}
fn reference(id: Id) -> String {
    format!("blake3:{id}")
}
fn event_identity(cursor: &ProgressCursor, payload: Id, tick: u64) -> Result<String, Error> {
    Ok(reference(
        codec::identity("batch-progress.event.v1", &(cursor, payload, tick)).map_err(crate::invalid)?,
    ))
}
fn bounded<T: Serialize>(value: T) -> Result<T, Error> {
    if codec::encode(PROGRESS_SCHEMA, &value).map_err(crate::invalid)?.canonical.len() > MAX_PROGRESS_REPLY_BYTES {
        return Err(Error::Bound);
    }
    Ok(value)
}
impl Journal {
    pub(crate) fn bootstrap(state: &State, state_ref: Id) -> Result<Self, Error> {
        let mut journal = Self {
            service_id: format!("batch-progress:{}", state.cluster),
            epoch: state.generation,
            revision: 0,
            first_retained_revision: 0,
            state_ref: None,
            events: Vec::new(),
        };
        journal.advance(state, state_ref, progress_delta(None, state)?)?;
        Ok(journal)
    }
    fn core(&self) -> stream::CommittedChangeStream {
        let mut core = stream::new_stream(&self.service_id, self.epoch).expect("validated durable stream namespace");
        core.revision = self.revision;
        core.first_retained_revision = self.first_retained_revision;
        core.state_ref = self.state_ref.map(reference);
        core.events = self
            .events
            .iter()
            .map(|e| {
                let mut event = stream::CommittedChangeEvent {
                    event_ref: String::new(),
                    service_id: e.cursor.service_id.clone(),
                    stream_epoch: e.cursor.stream_epoch,
                    revision: e.cursor.revision,
                    ordinal: e.cursor.ordinal,
                    payload_ref: reference(e.payload_ref),
                    committed_at_tick: e.committed_at_tick,
                };
                event.event_ref = stream::committed_change_identity(&event);
                event
            })
            .collect();
        core
    }
    pub(crate) fn validate(&self, state: &State, expected: Id) -> Result<(), Error> {
        if self.service_id != format!("batch-progress:{}", state.cluster)
            || self.epoch != state.generation
            || self.state_ref.as_ref() != Some(&expected)
            || self.events.len() > MAX_BATCH_PROGRESS_EVENTS
        {
            return Err(Error::Invalid("progress/state cohort mismatch".into()));
        }
        let mut position = None;
        for event in &self.events {
            let key = (event.cursor.revision, event.cursor.ordinal);
            if event.cursor.service_id != self.service_id
                || event.cursor.stream_epoch != self.epoch
                || key.0 > self.revision
                || position.is_some_and(|old| old >= key)
                || event_identity(&event.cursor, event.payload_ref, event.committed_at_tick)? != event.event_ref
                || codec::identity("batch-progress.payload.v1", &event.payload).map_err(crate::invalid)?
                    != event.payload_ref
            {
                return Err(Error::Invalid("corrupt durable progress".into()));
            }
            position = Some(key);
        }
        Ok(())
    }
    pub(crate) fn advance(&mut self, next: &State, state_ref: Id, payloads: Vec<ProgressPayload>) -> Result<(), Error> {
        self.state_ref = Some(state_ref);
        if payloads.is_empty() {
            return Ok(());
        }
        let mut core = self.core();
        let state_reference = reference(state_ref);
        for payload in payloads {
            let payload_ref = codec::identity("batch-progress.payload.v1", &payload).map_err(crate::invalid)?;
            let committed = stream::commit_transition(&mut core, &stream::CommitInput {
                payload_ref: reference(payload_ref),
                state_ref: state_reference.clone(),
                committed_at_tick: next.tick,
                event_count: 1,
            })
            .map_err(|issue| crate::invalid(issue.as_str()))?;
            let e = committed.into_iter().next().ok_or(Error::Unknown)?;
            let cursor = ProgressCursor {
                service_id: e.service_id,
                stream_epoch: e.stream_epoch,
                revision: e.revision,
                ordinal: e.ordinal,
            };
            self.events.push(ProgressEvent {
                event_ref: event_identity(&cursor, payload_ref, e.committed_at_tick)?,
                cursor,
                payload_ref,
                committed_at_tick: e.committed_at_tick,
                payload,
            });
        }
        self.revision = core.revision;
        self.first_retained_revision = core.first_retained_revision;
        // The application selects a smaller retained window than the reusable
        // core ceiling; cursor and gap decisions remain owned by that core.
        if self.events.len() > MAX_BATCH_PROGRESS_EVENTS {
            let overflow = self.events.len() - MAX_BATCH_PROGRESS_EVENTS;
            self.events.drain(..overflow);
            self.first_retained_revision = self.events[0].cursor.revision;
        }
        Ok(())
    }
    // r[impl molten.observations.cursors]
    pub(crate) fn snapshot(&self, state: &State, run_id: Id, state_ref: Id) -> Result<ProgressSnapshot, Error> {
        let run = state.runs.get(&run_id).ok_or(Error::Unavailable)?;
        bounded(ProgressSnapshot {
            schema: PROGRESS_SCHEMA.into(),
            cluster: state.cluster,
            generation: state.generation,
            run: run.id,
            spec_ref: run.spec_ref,
            phase: run.phase(),
            cancel_requested: run.cancel_requested,
            tasks: progress_tasks(state, run)?,
            state_ref,
            state_revision: state.revision,
            watermark: stream::snapshot_with_watermark(&self.core()).watermark.into(),
        })
    }
    // r[impl molten.observations.cursors]
    pub(crate) fn read(&self, cursor: ProgressCursor, limit: u32) -> Result<ProgressPage, Error> {
        if limit == 0 || limit > MAX_PROGRESS_PAGE || cursor.service_id.len() > 128 {
            return Err(Error::Bound);
        }
        let outcome = stream::subscribe(&self.core(), &(&cursor).into(), limit)
            .map_err(|issue| crate::invalid(issue.as_str()))?;
        bounded(match outcome {
            stream::SubscriptionOutcome::Events { events, next_cursor } => {
                let events = events
                    .iter()
                    .map(|event| {
                        let index = self
                            .events
                            .binary_search_by_key(&(event.revision, event.ordinal), |e| {
                                (e.cursor.revision, e.cursor.ordinal)
                            })
                            .map_err(|_| Error::Unknown)?;
                        Ok(self.events[index].clone())
                    })
                    .collect::<Result<Vec<_>, Error>>()?;
                ProgressPage::Events {
                    events,
                    next: next_cursor.into(),
                }
            }
            stream::SubscriptionOutcome::RetentionGap { resnapshot_from } => ProgressPage::ResnapshotRequired {
                from: resnapshot_from.into(),
            },
            stream::SubscriptionOutcome::StaleEpoch { current_epoch } => ProgressPage::StaleEpoch { current_epoch },
            stream::SubscriptionOutcome::ForeignService { .. } => ProgressPage::ForeignService,
        })
    }
}
