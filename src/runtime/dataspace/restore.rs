use preserves::ValueImpl;

use super::*;

impl RuntimeState {
    /// Reconstruct committed state only. This does not restore authority, activate
    /// handlers, run facet cleanup, or replay outstanding effects.
    pub fn from_snapshot(snapshot: RuntimeSnapshot) -> Result<Self> {
        snapshot.validate_for_restore()?;
        let mut state = Self::new(snapshot.rng_state);
        state.overwrite_from_snapshot(snapshot);
        Ok(state)
    }
}

impl RuntimeSnapshot {
    pub(crate) fn validate_for_restore(&self) -> Result<()> {
        if self.rng_state == 0
            || self.facets.len() > MAX_FACETS
            || self.logical_time == u64::MAX
            || self.effect_sequence == u64::MAX
            || self.messages.len() > 4096
            || self.assertions.len() > 4096
            || self.observers.len() > 4096
        {
            return Err(MoltenError::invalid_harness(
                "invalid runtime snapshot RNG, exhausted counter or collection bound",
            ));
        }
        for (key, facet) in &self.facets {
            validate_facet_token("facet id", key)?;
            if key != &facet.facet_id {
                return Err(MoltenError::invalid_harness("runtime snapshot facet key mismatch"));
            }
            if let Some(handler) = &facet.stop_handler_ref {
                validate_facet_token("facet stop handler", handler)?;
            }
            let mut cursor = facet.parent.as_ref();
            let mut depth = 1;
            while let Some(parent_id) = cursor {
                depth += 1;
                if depth > MAX_FACET_DEPTH || parent_id == key {
                    return Err(MoltenError::invalid_harness("runtime snapshot facet cycle or depth exceeded"));
                }
                let parent = self
                    .facets
                    .get(parent_id)
                    .ok_or_else(|| MoltenError::invalid_harness("runtime snapshot missing parent facet"))?;
                if parent.actor != facet.actor
                    || (facet.state == FacetState::Running && parent.state != FacetState::Running)
                {
                    return Err(MoltenError::invalid_harness("runtime snapshot invalid parent ownership/state"));
                }
                cursor = parent.parent.as_ref();
            }
        }
        for assertion in &self.assertions {
            if let Some(facet) = &assertion.facet {
                self.require_running_facet(&assertion.actor, facet)?;
            }
        }
        for observer in &self.observers {
            if let Some(facet) = &observer.facet {
                self.require_running_facet(&observer.actor, facet)?;
            }
        }
        Ok(())
    }

    fn require_running_facet(&self, actor: &str, id: &str) -> Result<()> {
        let facet = self
            .facets
            .get(id)
            .ok_or_else(|| MoltenError::invalid_harness("runtime snapshot scope facet missing"))?;
        if facet.actor != actor || facet.state != FacetState::Running {
            return Err(MoltenError::invalid_harness("runtime snapshot scope facet ownership/state mismatch"));
        }
        Ok(())
    }
}

impl RuntimeSnapshot {
    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self> {
        let decoded = crate::preserves_rail::strict_canonical_decode(bytes)?;
        let fields = snapshot_fields(&decoded.value, "runtime-snapshot-v1", 7)?;
        let mut snapshot = Self {
            logical_time: number(&fields[0])?,
            rng_state: number(&fields[1])?,
            effect_sequence: number(&fields[2])?,
            messages: OrderedSet::new(),
            assertions: OrderedSet::new(),
            observers: OrderedSet::new(),
            facets: OrderedMap::new(),
        };
        for value in sequence_values(&fields[3])? {
            let f = snapshot_fields(&value, "runtime-message-v1", 4)?;
            snapshot.messages.insert(Message {
                from: text(&f[0])?,
                to: text(&f[1])?,
                body: Value::new(f[2].clone())?,
            });
        }
        for value in sequence_values(&fields[4])? {
            let f = snapshot_fields(&value, "runtime-assertion-v1", 4)?;
            snapshot.assertions.insert(Assertion {
                actor: text(&f[0])?,
                value: Value::new(f[1].clone())?,
                facet: optional_text(&f[3])?,
            });
        }
        for value in sequence_values(&fields[5])? {
            let f = snapshot_fields(&value, "runtime-observer-v1", 4)?;
            snapshot.observers.insert(Observer {
                actor: text(&f[0])?,
                pattern: Value::new(f[1].clone())?,
                facet: optional_text(&f[3])?,
            });
        }
        for value in sequence_values(&fields[6])? {
            let f = snapshot_fields(&value, "runtime-facet-v1", 5)?;
            let facet = Facet {
                facet_id: text(&f[0])?,
                actor: text(&f[1])?,
                parent: optional_text(&f[2])?,
                state: match text(&f[3])?.as_str() {
                    "running" => FacetState::Running,
                    "stopped" => FacetState::Stopped,
                    _ => return Err(MoltenError::invalid_harness("unknown runtime facet state")),
                },
                stop_handler_ref: optional_text(&f[4])?,
            };
            if snapshot.facets.insert(facet.facet_id.clone(), facet).is_some() {
                return Err(MoltenError::invalid_harness("duplicate runtime snapshot facet"));
            }
        }
        // Re-encoding checks value references, order, duplicate set members and
        // every closed record field, not just Preserves wire canonicality.
        if crate::preserves_rail::canonical_bytes(&snapshot.to_value())? != bytes {
            return Err(MoltenError::invalid_harness("runtime snapshot is not normalized or has crossed value refs"));
        }
        snapshot.validate_for_restore()?;
        Ok(snapshot)
    }
}

fn snapshot_fields(value: &IoValue, label: &str, arity: u64) -> Result<Vec<IoValue>> {
    Ok(crate::preserves_rail::simple_record_fields(value, label, arity)?
        .fields_iter()
        .map(crate::preserves_rail::value_to_iovalue)
        .collect())
}

fn sequence_values(value: &IoValue) -> Result<Vec<IoValue>> {
    let values = value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("expected runtime snapshot sequence"))?;
    if values.len() > 4096 {
        return Err(MoltenError::invalid_harness("runtime snapshot sequence exceeds bound"));
    }
    Ok(values.as_slice().iter().map(crate::preserves_rail::value_to_iovalue).collect())
}

fn text(value: &IoValue) -> Result<String> {
    value
        .as_string()
        .map(|text| text.to_string())
        .ok_or_else(|| MoltenError::invalid_harness("expected runtime snapshot text"))
}

fn optional_text(value: &IoValue) -> Result<Option<String>> {
    let value = text(value)?;
    Ok((!value.is_empty()).then_some(value))
}

fn number(value: &IoValue) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness("expected runtime snapshot u64"))?
        .map_err(|error| MoltenError::invalid_harness(format!("runtime snapshot integer: {error}")))
}
