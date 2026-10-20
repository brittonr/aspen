impl RuntimeState {
    fn has_visible_assertion(&self, value: &Value) -> bool {
        self.assertions.iter().any(|assertion| assertion.value == *value)
    }

    fn holds_assertion(&self, actor: &str, facet: Option<&str>, value: &Value) -> bool {
        self.assertions
            .iter()
            .any(|assertion| assertion.actor == actor && assertion.facet.as_deref() == facet && assertion.value == *value)
    }

    fn visible_owner_count(&self, value: &Value) -> usize {
        self.assertions.iter().filter(|assertion| assertion.value == *value).count()
    }
}

impl RuntimeState {
    pub fn apply_facet_create(
        &mut self,
        actor: &str,
        facet_id: &str,
        parent: Option<&str>,
        stop_handler_ref: Option<&str>,
    ) -> Result<(Vec<Event>, PredicateReceipt)> {
        self.validate_new_facet(actor, facet_id, parent)?;
        if let Some(handler) = stop_handler_ref {
            validate_facet_token("facet stop handler", handler)?;
        }
        let facet = Facet {
            facet_id: facet_id.to_string(),
            actor: actor.to_string(),
            parent: parent.map(str::to_string),
            state: FacetState::Running,
            stop_handler_ref: stop_handler_ref.map(str::to_string),
        };
        let mut turn = PendingTurn::new();
        turn.events.push(Event::FacetCreated {
            facet_id: facet_id.to_string(),
            actor: actor.to_string(),
            parent: parent.map(str::to_string),
        });
        turn.actions.push(TurnAction::FacetCreate(facet));
        self.commit_turn_with_predicate_receipt(turn)
    }

    pub fn apply_facet_assert(
        &mut self,
        actor: &str,
        facet_id: &str,
        value: &Value,
    ) -> Result<(Vec<Event>, PredicateReceipt)> {
        self.require_running_facet(actor, facet_id)?;
        let mut turn = PendingTurn::new();
        self.stage_assert(&mut turn, actor, value, Some(facet_id));
        self.commit_turn_with_predicate_receipt(turn)
    }

    pub fn apply_facet_observe(
        &mut self,
        actor: &str,
        facet_id: &str,
        pattern: &Value,
    ) -> Result<(Vec<Event>, PredicateReceipt)> {
        self.require_running_facet(actor, facet_id)?;
        let mut turn = PendingTurn::new();
        self.stage_observe(&mut turn, actor, pattern, Some(facet_id));
        self.commit_turn_with_predicate_receipt(turn)
    }

    pub fn apply_facet_retract(
        &mut self,
        actor: &str,
        facet_id: &str,
        value: &Value,
    ) -> Result<(Vec<Event>, PredicateReceipt)> {
        self.require_running_facet(actor, facet_id)?;
        let mut turn = PendingTurn::new();
        self.stage_retract(&mut turn, actor, value, Some(facet_id));
        self.commit_turn_with_predicate_receipt(turn)
    }

    pub fn apply_facet_stop(
        &mut self,
        actor: &str,
        facet_id: &str,
    ) -> Result<(Vec<Event>, PredicateReceipt)> {
        self.require_running_facet(actor, facet_id)?;
        let mut turn = PendingTurn::new();
        self.stage_facet_stop(&mut turn, facet_id)?;
        self.commit_turn_with_predicate_receipt(turn)
    }

    fn stage_facet_stop(&self, turn: &mut PendingTurn, facet_id: &str) -> Result<()> {
        for id in self.facet_stop_order(facet_id)? {
            self.stage_single_facet_stop(turn, &id);
        }
        Ok(())
    }

    fn facet_stop_order(&self, facet_id: &str) -> Result<Vec<String>> {
        let mut order = Vec::new();
        let mut stack = vec![(facet_id.to_string(), false)];
        while let Some((id, expanded)) = stack.pop() {
            if expanded {
                crate::bounded::push(&mut order, id, MAX_FACETS, "facet stop order")?;
                continue;
            }
            crate::bounded::push(&mut stack, (id.clone(), true), MAX_FACETS.saturating_mul(2), "facet stop stack")?;
            let mut children = self
                .facets
                .iter()
                .filter(|(_, facet)| facet.parent.as_deref() == Some(id.as_str()) && facet.state == FacetState::Running)
                .map(|(child, _)| child.to_string())
                .collect::<Vec<_>>();
            children.sort();
            for child in children.into_iter().rev() {
                crate::bounded::push(&mut stack, (child, false), MAX_FACETS.saturating_mul(2), "facet stop stack")?;
            }
        }
        Ok(order)
    }

    fn stage_single_facet_stop(&self, turn: &mut PendingTurn, facet_id: &str) {
        for assertion in self.assertions.iter().filter(|assertion| assertion.facet.as_deref() == Some(facet_id)) {
            turn.events.push(Event::AssertionRetracted {
                actor: assertion.actor.clone(),
                value: assertion.value.clone(),
            });
            turn.actions.push(TurnAction::Retract(assertion.clone()));
        }
        for observer in self.observers.iter().filter(|observer| observer.facet.as_deref() == Some(facet_id)) {
            turn.actions.push(TurnAction::RemoveObserver(observer.clone()));
        }
        turn.events.push(Event::FacetStopped {
            facet_id: facet_id.to_string(),
        });
        turn.actions.push(TurnAction::FacetStop(facet_id.to_string()));
        if let Some(handler) = self.facets.get(facet_id).and_then(|facet| facet.stop_handler_ref.as_ref()) {
            turn.events.push(Event::FacetStopHandlerRan {
                facet_id: facet_id.to_string(),
                handler_ref: handler.clone(),
            });
        }
    }

    fn require_running_facet(&self, actor: &str, facet_id: &str) -> Result<()> {
        let facet = self.facets.get(facet_id).ok_or_else(|| {
            crate::error::Failure::invalid_harness(format!("unknown runtime facet {facet_id}"))
        })?;
        if facet.actor != actor {
            return Err(crate::error::Failure::invalid_harness(format!(
                "runtime facet {facet_id} is not owned by actor {actor}"
            )));
        }
        if facet.state != FacetState::Running {
            return Err(crate::error::Failure::invalid_harness(format!(
                "runtime facet {facet_id} is stopped and cannot accept work"
            )));
        }
        Ok(())
    }

    fn validate_new_facet(&self, actor: &str, facet_id: &str, parent: Option<&str>) -> Result<()> {
        validate_facet_token("facet id", facet_id)?;
        if self.facets.contains_key(facet_id) {
            return Err(crate::error::Failure::invalid_harness(format!(
                "runtime facet {facet_id} already exists"
            )));
        }
        if self.facets.len() >= MAX_FACETS {
            return Err(crate::error::Failure::invalid_harness("runtime facet bound exceeded"));
        }
        if let Some(parent_id) = parent {
            let parent = self.facets.get(parent_id).ok_or_else(|| {
                crate::error::Failure::invalid_harness(format!("unknown parent runtime facet {parent_id}"))
            })?;
            if parent.actor != actor {
                return Err(crate::error::Failure::invalid_harness(format!(
                    "parent runtime facet {parent_id} is not owned by actor {actor}"
                )));
            }
            if parent.state != FacetState::Running {
                return Err(crate::error::Failure::invalid_harness(format!(
                    "parent runtime facet {parent_id} is stopped"
                )));
            }
            if self.facet_depth(parent_id) >= MAX_FACET_DEPTH {
                return Err(crate::error::Failure::invalid_harness("runtime facet depth bound exceeded"));
            }
        }
        Ok(())
    }

    fn facet_depth(&self, facet_id: &str) -> usize {
        let mut depth = 1;
        let mut cursor = self.facets.get(facet_id).and_then(|facet| facet.parent.clone());
        while let Some(parent) = cursor {
            depth += 1;
            cursor = self.facets.get(&parent).and_then(|facet| facet.parent.clone());
        }
        depth
    }}

fn validate_facet_token(label: &str, value: &str) -> Result<()> {
    if value.is_empty() {
        return Err(crate::error::Failure::invalid_harness(format!("{label} must not be empty")));
    }
    if value.len() > MAX_FACET_ID_BYTES {
        return Err(crate::error::Failure::invalid_harness(format!(
            "{label} exceeds {MAX_FACET_ID_BYTES} bytes"
        )));
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b':' | b'.'))
    {
        return Err(crate::error::Failure::invalid_harness(format!(
            "{label} must use ASCII alphanumeric, underscore, dash, colon, or dot bytes"
        )));
    }
    Ok(())
}
