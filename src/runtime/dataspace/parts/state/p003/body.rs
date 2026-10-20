const MAX_FACET_ID_BYTES: usize = 128;
const MAX_FACET_DEPTH: usize = 32;
const MAX_FACETS: usize = 1024;

impl RuntimeState {
    fn stage_step(&self, turn: &mut PendingTurn, step: &Step) {
        match step {
            Step::Send { from, to, body } => {
                let message = Message {
                    from: from.clone(),
                    to: to.clone(),
                    body: body.clone(),
                };
                turn.events.push(Event::MessageDelivered {
                    from: from.clone(),
                    to: to.clone(),
                    body: body.clone(),
                });
                turn.actions.push(TurnAction::Send(message));
            }
            Step::Observe { actor, pattern } => self.stage_observe(turn, actor, pattern, None),
            Step::Assert { actor, value } => self.stage_assert(turn, actor, value, None),
            Step::Retract { actor, value } => self.stage_retract(turn, actor, value, None),
            Step::Clock { .. } | Step::Random { .. } => {}
        }
    }

    fn stage_observe(&self, turn: &mut PendingTurn, actor: &str, pattern: &Value, facet: Option<&str>) {
        let observer = Observer {
            actor: actor.to_string(),
            pattern: pattern.clone(),
            facet: facet.map(str::to_string),
        };
        turn.events.push(Event::ObserveRegistered {
            actor: actor.to_string(),
            pattern: pattern.clone(),
        });
        let mut notified = OrderedSet::new();
        for assertion in self.assertions.iter().filter(|assertion| observe_value_matches(pattern, &assertion.value)) {
            if !notified.insert(assertion.value.clone()) {
                continue;
            }
            // r[impl molten.preserves_boundary_codegen.pattern_routing]
            turn.events.push(Event::AssertionObserved {
                observer: actor.to_string(),
                owner: assertion.actor.clone(),
                value: assertion.value.clone(),
            });
        }
        turn.actions.push(TurnAction::Observe(observer));
    }

    fn stage_assert(&self, turn: &mut PendingTurn, actor: &str, value: &Value, facet: Option<&str>) {
        let assertion = Assertion {
            actor: actor.to_string(),
            value: value.clone(),
            facet: facet.map(str::to_string),
        };
        let is_newly_visible = !self.has_visible_assertion(value);
        turn.events.push(Event::AssertionCommitted {
            actor: actor.to_string(),
            value: value.clone(),
        });
        if is_newly_visible {
            for observer in self.observers.iter().filter(|observer| observe_value_matches(&observer.pattern, value)) {
                // r[impl molten.preserves_boundary_codegen.pattern_routing]
                turn.events.push(Event::AssertionObserved {
                    observer: observer.actor.clone(),
                    owner: actor.to_string(),
                    value: value.clone(),
                });
            }
        }
        turn.actions.push(TurnAction::Assert(assertion));
    }

    fn stage_retract(&self, turn: &mut PendingTurn, actor: &str, value: &Value, facet: Option<&str>) {
        if !self.holds_assertion(actor, facet, value) {
            turn.events.push(Event::TurnRolledBack {
                actor: actor.to_string(),
                reason: RETRACTION_WITHOUT_ASSERTION.to_string(),
            });
            return;
        }
        let assertion = Assertion {
            actor: actor.to_string(),
            value: value.clone(),
            facet: facet.map(str::to_string),
        };
        let is_last_owner = self.visible_owner_count(value) == 1;
        turn.events.push(Event::AssertionRetracted {
            actor: actor.to_string(),
            value: value.clone(),
        });
        if is_last_owner {
            for observer in self.observers.iter().filter(|observer| observe_value_matches(&observer.pattern, value)) {
                // r[impl molten.preserves_boundary_codegen.pattern_routing]
                turn.events.push(Event::AssertionRetractionObserved {
                    observer: observer.actor.clone(),
                    owner: actor.to_string(),
                    value: value.clone(),
                });
            }
        }
        turn.actions.push(TurnAction::Retract(assertion));
    }

}
