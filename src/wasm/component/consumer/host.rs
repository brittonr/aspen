//! Owned live and offline effect adapters for product composition roots.

use super::super::EffectRequestHost;
use crate::system_extension::NativeCallbackValuePort;

mod publication;
mod state;
pub use state::Publication;
pub use state::RecordedEffect;
pub use state::Recording;
pub use state::Records;

const MAX_RECORDED_EFFECTS: usize = 1_024;

/// Own the current product authority services, constructing borrowed boundaries
/// only for the duration of an invocation. None of these facts come from Wasm.
pub trait CurrentContext {
    fn with_current<R>(&mut self, use_current: impl FnOnce(super::super::host::CurrentInput<'_>) -> R) -> R;
}

/// Share an existing product value store, rather than copying it into a guest host.
#[derive(Clone)]
pub struct SharedValuePort(pub crate::system_extension::SharedCallbackValuePort);

impl NativeCallbackValuePort for SharedValuePort {
    fn materialize(
        &mut self,
        reference: &str,
        maximum: u64,
    ) -> std::result::Result<crate::system_extension::NativeCallbackValue, crate::system_extension::ValuePortFailure>
    {
        self.0
            .lock()
            .map_err(|_| {
                crate::system_extension::ValuePortFailure::new(
                    crate::system_extension::ValuePortFailureKind::RejectedBeforeAcceptance,
                    "component value port lock poisoned",
                )
            })?
            .materialize(reference, maximum)
    }

    fn publish(
        &mut self,
        value: &crate::system_extension::NativeCallbackValue,
        maximum: u64,
    ) -> std::result::Result<crate::system_extension::ValuePublicationReceipt, crate::system_extension::ValuePortFailure>
    {
        self.0
            .lock()
            .map_err(|_| {
                crate::system_extension::ValuePortFailure::new(
                    crate::system_extension::ValuePortFailureKind::UnknownAfterAcceptance,
                    "component value port lock poisoned",
                )
            })?
            .publish(value, maximum)
    }
}

/// Owned lifecycle, current authority services and value port for one live turn.
pub struct LiveInput<C, V> {
    pub scope: super::super::host::Scope,
    pub generation: u64,
    pub incarnation: u64,
    pub current: C,
    pub values: V,
}

pub struct Live<C, V> {
    session: super::super::host::Session,
    current: C,
    values: publication::RecordingValuePort<V>,
    recording: Recording,
}

impl<C: CurrentContext, V: NativeCallbackValuePort> Live<C, V> {
    pub fn new(
        scope: super::super::host::Scope,
        generation: u64,
        incarnation: u64,
        current: C,
        values: V,
    ) -> super::super::Result<(Self, Recording)> {
        let session = super::super::host::Session::new(scope, generation, incarnation)?;
        let recording = Recording::with_capacity(MAX_RECORDED_EFFECTS, None);
        Ok((
            Self {
                session,
                current,
                values: publication::RecordingValuePort::new(values, recording.clone()),
                recording: recording.clone(),
            },
            recording,
        ))
    }

    pub(crate) fn callback_binding(&mut self) -> (super::super::ImportGrant, Recording) {
        let grant = self.current.with_current(|current| current.grant.clone());
        (grant, self.recording.clone())
    }

    fn dispatch(&mut self, bytes: &[u8]) -> super::super::Result<Vec<u8>> {
        self.recording.admit_boundary()?;
        self.values.begin_boundary(bytes);
        let outcome = self.current.with_current(|current| self.session.invoke(bytes, &current, &mut self.values))?;
        let response_bytes = outcome.response_bytes;
        self.recording.retain(RecordedEffect {
            record: outcome.record,
            response_bytes: response_bytes.clone(),
            entry: outcome.entry,
        })?;
        Ok(response_bytes)
    }
}

impl<C: CurrentContext, V: NativeCallbackValuePort> EffectRequestHost for Live<C, V> {
    fn invoke(&mut self, bytes: &[u8]) -> super::super::Result<Vec<u8>> {
        let result = self.dispatch(bytes);
        if let Err(denial) = &result {
            self.recording.fail(denial);
        }
        result
    }
}

/// This adapter has no current authority or live value port: no fallback exists.
pub struct Replay {
    replay: super::super::host::Replay,
    scope: super::super::host::Scope,
    effects: std::vec::IntoIter<RecordedEffect>,
    recording: Recording,
}

impl Replay {
    pub fn new(
        scope: super::super::host::Scope,
        generation: u64,
        incarnation: u64,
        effects: Vec<RecordedEffect>,
    ) -> super::super::Result<(Self, Recording)> {
        let maximum_effects = effects.len();
        if maximum_effects > MAX_RECORDED_EFFECTS {
            return Err(super::super::Denial::new("component replay exceeds the bounded host call cohort"));
        }
        let replay = super::super::host::Replay::new(scope.clone(), generation, incarnation)?;
        let recording = Recording::with_capacity(maximum_effects, Some(maximum_effects));
        Ok((
            Self {
                replay,
                scope,
                effects: effects.into_iter(),
                recording: recording.clone(),
            },
            recording,
        ))
    }

    pub(crate) fn callback_recording(&self) -> Recording {
        self.recording.clone()
    }

    fn consume(&mut self, bytes: &[u8]) -> super::super::Result<Vec<u8>> {
        self.recording.admit_boundary()?;
        let effect = self
            .effects
            .next()
            .ok_or_else(|| super::super::Denial::new("component replay requested an unrecorded effect"))?;
        self.replay.consume(super::super::host::ReplayInput {
            request_bytes: bytes,
            expected_record_ref: &effect.record.value_ref,
            record: &effect.record,
            response_bytes: &effect.response_bytes,
        })?;
        let request = super::super::decode_host_request(bytes)?;
        if effect.entry.sequence != request.sequence()
            || effect.entry.request_ref != request.request_ref()
            || effect.entry.response_ref != crate::preserves_rail::content_ref_from_bytes(&effect.response_bytes)
            || effect.entry.effect_kind != "blob-get"
            || effect.entry.run_identity_ref != self.scope.run_ref
            || effect.entry.handler_profile_ref != self.scope.handler_profile_ref
            || effect.entry.turn_ref != self.scope.turn_ref
            || effect.entry.boundary_ref != self.scope.boundary_ref
        {
            return Err(super::super::Denial::new(
                "component replay effect-log entry differs from its recorded boundary",
            ));
        }
        let response = effect.response_bytes.clone();
        self.recording.retain(effect)?;
        Ok(response)
    }
}

impl EffectRequestHost for Replay {
    fn invoke(&mut self, bytes: &[u8]) -> super::super::Result<Vec<u8>> {
        let result = self.consume(bytes);
        if let Err(denial) = &result {
            self.recording.fail(denial);
        }
        result
    }
}

fn poisoned_recording() -> super::super::Denial {
    super::super::Denial::new("component host recording unavailable; completed effects require reconciliation")
}
