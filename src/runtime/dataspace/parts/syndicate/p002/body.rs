impl ReferenceHarness {
    fn has_assertion_value(&self, value: &Value) -> bool {
        self.assertions.keys().any(|assertion| assertion.value == *value)
    }

    fn holds_assertion(&self, actor: &str, value: &Value) -> bool {
        self.assertions.keys().any(|assertion| assertion.actor == actor && assertion.value == *value)
    }

    fn visible_owner_count(&self, value: &Value) -> usize {
        self.assertions.keys().filter(|assertion| assertion.value == *value).count()
    }

    fn apply_assert(&mut self, actor: &str, value: &Value) -> Result<Vec<Event>> {
        let assertion = Assertion {
            actor: actor.to_string(),
            value: value.clone(),
            facet: None,
        };
        let is_newly_visible = !self.has_assertion_value(value);
        let mut events = vec![Event::AssertionCommitted {
            actor: actor.to_string(),
            value: value.clone(),
        }];
        if is_newly_visible {
            for observer in self.matching_observers(value)? {
                events.push(Event::AssertionObserved {
                    observer: observer.actor,
                    owner: actor.to_string(),
                    value: value.clone(),
                });
            }
        }
        if !self.holds_assertion(actor, value) {
            self.assertions.change(assertion, SYNDICATE_BAG_INSERT_DELTA);
        }
        Ok(events)
    }

    fn apply_retract(&mut self, actor: &str, value: &Value) -> Result<Vec<Event>> {
        if !self.holds_assertion(actor, value) {
            return Ok(vec![Event::TurnRolledBack {
                actor: actor.to_string(),
                reason: RETRACTION_WITHOUT_ASSERTION.to_string(),
            }]);
        }
        let assertion = Assertion {
            actor: actor.to_string(),
            value: value.clone(),
            facet: None,
        };
        let is_last_owner = self.visible_owner_count(value) == 1;
        let mut events = vec![Event::AssertionRetracted {
            actor: actor.to_string(),
            value: value.clone(),
        }];
        if is_last_owner {
            for observer in self.matching_observers(value)? {
                events.push(Event::AssertionRetractionObserved {
                    observer: observer.actor,
                    owner: actor.to_string(),
                    value: value.clone(),
                });
            }
        }
        self.assertions.change_clamped(assertion, SYNDICATE_BAG_RETRACT_DELTA);
        Ok(events)
    }

    fn matching_assertions(&self, pattern_value: &Value) -> Result<Vec<Assertion>> {
        let pattern = RuntimePattern::from_observe_value(pattern_value)?;
        let mut assertions = Vec::with_capacity(self.assertions.len());
        let mut notified = OrderedSet::new();
        for assertion in self.assertions.keys() {
            if pattern.matches_value(&assertion.value)?.0 && notified.insert(assertion.value.clone()) {
                assertions.push(assertion.clone());
            }
        }
        Ok(assertions)
    }
}
