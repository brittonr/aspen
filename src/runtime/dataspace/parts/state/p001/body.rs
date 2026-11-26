
impl RuntimeState {
    pub fn new(seed: u64) -> Self {
        Self {
            logical_time: 0,
            rng_state: seed.max(1),
            effect_sequence: 0,
            messages: OrderedSet::new(),
            assertions: OrderedSet::new(),
            observers: OrderedSet::new(),
            facets: OrderedMap::new(),
        }
    }

    pub fn snapshot(&self) -> RuntimeSnapshot {
        RuntimeSnapshot {
            logical_time: self.logical_time,
            rng_state: self.rng_state,
            effect_sequence: self.effect_sequence,
            messages: self.messages.clone(),
            assertions: self.assertions.clone(),
            observers: self.observers.clone(),
            facets: self.facets.clone(),
        }
    }

    /// Inspect committed live assertions without cloning unrelated runtime state.
    pub fn live_assertions(&self) -> &OrderedSet<Assertion> {
        &self.assertions
    }

    pub fn apply_step(&mut self, step: &Step) -> Vec<Event> {
        match self.apply_step_with_predicate_receipt(step) {
            Ok((events, _receipt)) => events,
            Err(error) => self.rollback_turn(
                PendingTurn::new(),
                step.primary_actor().to_string(),
                format!("runtime predicate denied step: {error}"),
            ),
        }
    }

    /// Apply a step whose turn carries the given cause.
    ///
    /// The cause comes from the transition input. It is never derived from a
    /// wall clock, and it becomes part of the committed turn record.
    pub fn apply_step_with_cause(
        &mut self,
        step: &Step,
        cause: &super::super::RuntimeTurnCause,
    ) -> Result<(Vec<Event>, Option<PredicateReceipt>)> {
        match step {
            Step::Clock { actor } => Ok((self.local_clock_events(actor)?, None)),
            Step::Random { actor, upper } => Ok((self.local_random_events(actor, *upper)?, None)),
            Step::Send { .. } | Step::Observe { .. } | Step::Assert { .. } | Step::Retract { .. } => {
                let turn = self.begin_turn_with_cause(step, cause);
                let (events, receipt) = self.commit_turn_with_predicate_receipt(turn)?;
                Ok((events, Some(receipt)))
            }
        }
    }

    pub fn apply_step_with_predicate_receipt(&mut self, step: &Step) -> Result<(Vec<Event>, Option<PredicateReceipt>)> {
        match step {
            Step::Clock { actor } => Ok((self.local_clock_events(actor)?, None)),
            Step::Random { actor, upper } => Ok((self.local_random_events(actor, *upper)?, None)),
            Step::Send { .. } | Step::Observe { .. } | Step::Assert { .. } | Step::Retract { .. } => {
                let turn = self.begin_turn(step);
                let (events, receipt) = self.commit_turn_with_predicate_receipt(turn)?;
                Ok((events, Some(receipt)))
            }
        }
    }

    /// Answer a live clock effect. The sequence and logical time both advance
    /// through checked restorable counters before either is written, so an
    /// exhausted counter fails without a partial state change.
    fn local_clock_events(&mut self, actor: &str) -> Result<Vec<Event>> {
        let sequence = self.effect_sequence;
        let next_sequence = next_restorable_counter(sequence, "effect sequence")?;
        let value = self.logical_time;
        let next_logical_time = next_restorable_counter(value, "logical time")?;
        self.effect_sequence = next_sequence;
        self.logical_time = next_logical_time;
        let request = Event::EffectRequest {
            effect: Effect::Clock,
            actor: actor.to_string(),
            sequence,
            upper: None,
        };
        Ok(vec![request, effect_response(Effect::Clock, actor.to_string(), sequence, None, value)])
    }

    /// Answer a live random effect from the seeded stream. The value range is
    /// `0..upper`, the same range a recorded response must satisfy, so every
    /// live answer replays.
    fn local_random_events(&mut self, actor: &str, upper: u64) -> Result<Vec<Event>> {
        let sequence = self.effect_sequence;
        let next_sequence = next_restorable_counter(sequence, "effect sequence")?;
        let (next_state, value) = deterministic_random_step(self.rng_state, upper);
        validate_random_value(upper, value)?;
        self.effect_sequence = next_sequence;
        // Deterministic xorshift64* profile; not cryptographic and not ambient entropy.
        self.rng_state = next_state;
        let request = Event::EffectRequest {
            effect: Effect::Random,
            actor: actor.to_string(),
            sequence,
            upper: Some(upper),
        };
        Ok(vec![request, effect_response(Effect::Random, actor.to_string(), sequence, Some(upper), value)])
    }

    pub fn begin_turn(&self, step: &Step) -> PendingTurn {
        self.begin_turn_with_cause(step, &super::super::RuntimeTurnCause::external("runtime-step"))
    }

    /// Begin a turn whose cause names why the turn runs.
    pub fn begin_turn_with_cause(
        &self,
        step: &Step,
        cause: &super::super::RuntimeTurnCause,
    ) -> PendingTurn {
        let mut turn = PendingTurn::with_cause(step.primary_actor().to_string(), cause.clone());
        self.stage_step(&mut turn, step);
        turn
    }

    pub(crate) fn commit_turn(&mut self, turn: PendingTurn) -> Result<Vec<Event>> {
        let after = committed_turn_snapshot(&self.snapshot(), &turn);
        self.overwrite_from_snapshot(after);
        let mut events = turn.events;
        let mut event_refs = Vec::with_capacity(events.len());
        for event in &events {
            event_refs.push(event.event_ref()?);
        }
        events.push(Event::TurnCommitted {
            actor: turn.actor,
            cause: turn.cause,
            event_refs,
        });
        Ok(events)
    }

    // r[impl molten.runtime_spine.dataspace_determinism_and_rollback_integrity]
    /// Commit a staged turn, or roll it back when staging already decided the
    /// turn cannot commit.
    ///
    /// A staged rollback (for example a retraction of an assertion the actor
    /// does not hold) leaves state untouched and returns only the
    /// `TurnRolledBack` event with a receipt evaluated for the rolled-back
    /// outcome; it never appends `TurnCommitted` or a committed-outcome receipt.
    pub fn commit_turn_with_predicate_receipt(&mut self, turn: PendingTurn) -> Result<(Vec<Event>, PredicateReceipt)> {
        if let Some((actor, reason)) = staged_rollback(&turn) {
            return self.rollback_turn_with_outcome(turn, actor, reason, TurnOutcome::RolledBack);
        }
        let before = self.snapshot();
        let mut preview = self.clone();
        let events = preview.commit_turn(turn.clone())?;
        let after = preview.snapshot();
        let receipt = evaluate_turn_transition(&before, &turn, &after, TurnOutcome::Committed)?;
        if receipt.decision == PredicateDecision::Pass {
            *self = preview;
            Ok((events, receipt))
        } else {
            Err(MoltenError::invalid_harness("runtime turn predicate denied commit"))
        }
    }

    pub fn rollback_turn(&self, _turn: PendingTurn, actor: impl Into<String>, reason: impl Into<String>) -> Vec<Event> {
        vec![Event::TurnRolledBack {
            actor: actor.into(),
            reason: reason.into(),
        }]
    }

    pub fn rollback_turn_with_predicate_receipt(
        &self,
        turn: PendingTurn,
        actor: impl Into<String>,
        reason: impl Into<String>,
    ) -> Result<(Vec<Event>, PredicateReceipt)> {
        self.rollback_turn_with_outcome(turn, actor, reason, TurnOutcome::Denied)
    }

    fn rollback_turn_with_outcome(
        &self,
        turn: PendingTurn,
        actor: impl Into<String>,
        reason: impl Into<String>,
        outcome: TurnOutcome,
    ) -> Result<(Vec<Event>, PredicateReceipt)> {
        let before = self.snapshot();
        let receipt = evaluate_turn_transition(&before, &turn, &before, outcome)?;
        let events = self.rollback_turn(turn, actor, reason);
        Ok((events, receipt))
    }

    pub fn cleanup_actor_scope(&mut self, actor: &str) -> Result<RuntimeScopeCleanup> {
        let mut assertion_refs = Vec::with_capacity(self.assertions.len());
        for assertion in self.assertions.iter().filter(|assertion| assertion.actor == actor) {
            assertion_refs.push(assertion.assertion_ref()?);
        }
        let mut observer_refs = Vec::with_capacity(self.observers.len());
        for observer in self.observers.iter().filter(|observer| observer.actor == actor) {
            observer_refs.push(observer.observer_ref()?);
        }
        let mut message_refs = Vec::with_capacity(self.messages.len());
        for message in self.messages.iter().filter(|message| message.from == actor || message.to == actor) {
            message_refs.push(message.message_ref()?);
        }
        assertion_refs.sort();
        observer_refs.sort();
        message_refs.sort();
        self.assertions.retain(|assertion| assertion.actor != actor);
        self.observers.retain(|observer| observer.actor != actor);
        self.messages.retain(|message| message.from != actor && message.to != actor);
        self.facets.retain(|_, facet| facet.actor != actor);
        Ok(RuntimeScopeCleanup {
            actor: actor.to_owned(),
            assertion_refs,
            observer_refs,
            message_refs,
            cause: super::super::RuntimeTurnCause::Cleanup,
        })
    }

    /// Begin a recorded effect: allocate its sequence through the checked
    /// restorable counter and return the request, or `None` for a dataspace turn.
    pub fn begin_effect_for_step(&mut self, step: &Step) -> Result<Option<Event>> {
        let (effect, actor, upper) = match step {
            Step::Clock { actor } => (Effect::Clock, actor, None),
            Step::Random { actor, upper } => (Effect::Random, actor, Some(*upper)),
            Step::Send { .. } | Step::Observe { .. } | Step::Assert { .. } | Step::Retract { .. } => return Ok(None),
        };
        let sequence = self.effect_sequence;
        self.effect_sequence = next_restorable_counter(sequence, "effect sequence")?;
        Ok(Some(Event::EffectRequest {
            effect,
            actor: actor.clone(),
            sequence,
            upper,
        }))
    }

    pub fn apply_recorded_effect_response(&mut self, request: &Event, value: u64) -> Result<Event> {
        let transition = recorded_effect_response_transition(&self.snapshot(), request, value)?;
        self.overwrite_from_snapshot(transition.after);
        Ok(transition.response)
    }

    fn overwrite_from_snapshot(&mut self, snapshot: RuntimeSnapshot) {
        self.logical_time = snapshot.logical_time;
        self.rng_state = snapshot.rng_state;
        self.effect_sequence = snapshot.effect_sequence;
        self.messages = snapshot.messages;
        self.assertions = snapshot.assertions;
        self.observers = snapshot.observers;
        self.facets = snapshot.facets;
    }

}

/// The rollback a turn's staging already decided, if any.
fn staged_rollback(turn: &PendingTurn) -> Option<(String, String)> {
    turn.events.iter().find_map(|event| match event {
        Event::TurnRolledBack { actor, reason } => Some((actor.clone(), reason.clone())),
        _ => None,
    })
}
