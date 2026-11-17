use super::super::*;

impl super::super::Table {
    /// An owner may hold at most one assertion per name. Changing ready
    /// retracts the old value and publishes the new value atomically.
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "The host exclusively owns this table; publish preflights capacity before a serialized assertion transition."
    )]
    pub fn publish(&mut self, facet: Facet, name: &str, ready: bool) -> Result<bool, Error> {
        let value = attempt!(self.value(facet, name, ready, Operation::Publish));
        let existing = self.owned_index(facet, name);
        let prior = existing.map(|index| self.assertions[index].service.ready);
        if decide_publication(prior, ready) == Publication::Unchanged {
            return Ok(true);
        }
        if existing.is_none() && self.assertions.len() >= self.limits.assertions {
            return Err(Error::Capacity);
        }
        let old_count = if let Some(index) = existing {
            let was_ready = self.assertions[index].service.ready;
            if self.asserted(name, was_ready, existing) {
                0
            } else {
                self.interested_count(name, was_ready)
            }
        } else {
            0
        };
        let is_new = !self.asserted(name, ready, None);
        let new_count = if is_new {
            self.interested_count(name, ready)
        } else {
            0
        };
        let Some(immediate) = old_count.checked_add(new_count) else {
            return Err(Error::Capacity);
        };
        let Some(remaining) = self.matches_count().checked_sub(old_count) else {
            return Err(Error::Capacity);
        };
        let Some(removals) = remaining.checked_add(new_count) else {
            return Err(Error::Capacity);
        };
        attempt!(self.reserve(super::Reservation {
            immediate,
            removals
        }));
        if let Some(index) = existing {
            let old = self.assertions.swap_remove(index);
            if !self.asserted(&old.service.name, old.service.ready, None) {
                self.notify(&old.service, Change::Removed);
            }
        }
        let service = Service::from_ref(value);
        if !self.asserted(&service.name, service.ready, None) {
            self.notify(&service, Change::Added);
        }
        self.assertions.push(Assertion {
            owner: facet,
            service,
        });
        Ok(true)
    }

    /// Install an exact-pair interest; return membership, not readiness.
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "This serialized host-owned table registers an interest only after rights and capacity checks."
    )]
    pub fn observe(&mut self, facet: Facet, name: &str, ready: bool) -> Result<bool, Error> {
        let value = attempt!(self.value(facet, name, ready, Operation::Observe));
        let is_present = self.asserted(name, ready, None);
        if self.interested(facet, name, ready) {
            return Ok(is_present);
        }
        if self.interests.len() >= self.limits.interests {
            return Err(Error::Capacity);
        }
        let matching = usize::from(is_present);
        let Some(removals) = self.matches_count().checked_add(matching) else {
            return Err(Error::Capacity);
        };
        attempt!(self.reserve(super::Reservation {
            immediate: matching,
            removals
        }));
        let service = Service::from_ref(value);
        if is_present {
            self.events.push(Event {
                observer: facet,
                service: service.clone(),
                change: Change::Added,
            });
        }
        self.interests.push(Interest {
            owner: facet,
            service,
        });
        Ok(is_present)
    }

    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "This serialized host-owned table retracts only the caller facet's assertion after authorization."
    )]
    pub fn retract(&mut self, facet: Facet, name: &str) -> Result<bool, Error> {
        attempt!(self.value(facet, name, false, Operation::Publish));
        let Some(index) = self.owned_index(facet, name) else {
            return Ok(false);
        };
        let old = &self.assertions[index].service;
        let is_last = !self.asserted(name, old.ready, Some(index));
        let count = if is_last {
            self.interested_count(name, old.ready)
        } else {
            0
        };
        let Some(removals) = self.matches_count().checked_sub(count) else {
            return Err(Error::Capacity);
        };
        attempt!(self.reserve(super::Reservation {
            immediate: count,
            removals
        }));
        let old = self.assertions.swap_remove(index);
        if !self.asserted(&old.service.name, old.service.ready, None) {
            self.notify(&old.service, Change::Removed);
        }
        Ok(true)
    }

    /// Hostile byte ingress must pass bounded Preserves decoding and the
    /// service schema before any facet/right lookup or typed state mutation.
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Wire ingress mutates only the exclusive host-owned table after bounded schema decoding."
    )]
    pub fn publish_wire(&mut self, facet: Facet, wire: &[u8]) -> Result<bool, Error> {
        Ok(attempt!(self.publish_wire_receipt(facet, wire)).0)
    }

    /// Return the *actual* validated view used in the committed transition,
    /// borrowed from caller bytes; no second parse or typed-value copy.
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "The receipt follows one bounded decode and one exclusive host-owned assertion commit."
    )]
    pub fn publish_wire_receipt<'a>(
        &mut self,
        facet: Facet,
        wire: &'a [u8],
    ) -> Result<(bool, ServiceRef<'a>), Error> {
        let value = attempt!(decode_service(wire, MAX_WIRE_DEPTH));
        Ok((
            attempt!(self.publish(facet, value.name, value.ready)),
            value,
        ))
    }

    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Wire ingress registers an interest only after bounded schema and facet-right checks."
    )]
    pub fn observe_wire(&mut self, facet: Facet, wire: &[u8]) -> Result<bool, Error> {
        let value = attempt!(decode_service(wire, MAX_WIRE_DEPTH));
        self.observe(facet, value.name, value.ready)
    }

    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Wire ingress retracts only the authenticated host facet's assertion after schema validation."
    )]
    pub fn retract_wire(&mut self, facet: Facet, wire: &[u8]) -> Result<bool, Error> {
        let value = attempt!(decode_service(wire, MAX_WIRE_DEPTH));
        self.retract(facet, value.name)
    }

    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "The exclusively borrowed table appends already-reserved scoped notifications; it never invokes a guest callback."
    )]
    fn notify(&mut self, service: &Service, change: Change) {
        let mut index = 0;
        while index < self.interests.len() {
            if self.interests[index].service == *service {
                let observer = self.interests[index].owner;
                self.events.push(Event {
                    observer,
                    service: service.clone(),
                    change,
                });
            }
            index += 1;
        }
    }

    /// Admission already reserved every removal event; trap cleanup cannot
    /// fail merely because accepted operations filled the event queue.
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Mandatory failure cleanup retires exclusive host-owned scopes and uses pre-reserved removal-event capacity."
    )]
    pub fn retire(&mut self, facet: Facet) -> Result<(), Error> {
        attempt!(self.scope(facet));
        let scopes = &self.scopes;
        let mut index = 0;
        while index < self.interests.len() {
            if Self::within(scopes, self.interests[index].owner, facet) {
                self.interests.remove(index);
            } else {
                index += 1;
            }
        }
        index = 0;
        while index < self.assertions.len() {
            if self.descendant(self.assertions[index].owner, facet) {
                let old = self.assertions.swap_remove(index);
                if !self.asserted(&old.service.name, old.service.ready, None) {
                    self.notify(&old.service, Change::Removed);
                }
            } else {
                index += 1;
            }
        }
        // Existing notifications for retired observers are no longer live.
        let scopes = &self.scopes;
        index = 0;
        while index < self.events.len() {
            if Self::within(scopes, self.events[index].observer, facet) {
                self.events.remove(index);
            } else {
                index += 1;
            }
        }
        index = 0;
        while index < self.scopes.len() {
            if Self::within(&self.scopes, self.scopes[index].id, facet) {
                self.scopes[index].live = false;
            }
            index += 1;
        }
        Ok(())
    }

    pub fn events(&self) -> &[Event] {
        &self.events
    }
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "The host exclusively owns notification storage and explicitly acknowledges already observed events."
    )]
    pub fn clear_events(&mut self) {
        self.events.clear();
    }
    pub fn counts(&self) -> (u32, u32, u32) {
        // Constructor limits (8/8/8) make each conversion exact.
        let mut live = 0;
        let mut index = 0;
        while index < self.scopes.len() {
            if self.scopes[index].live && !self.retired_ancestor(self.scopes[index].id) {
                live += 1;
            }
            index += 1;
        }
        (
            live,
            self.assertions.len() as u32,
            self.interests.len() as u32,
        )
    }

    /// Read-only exact-pair membership for host pre-export admission.
    pub fn contains_pair(&self, name: &str, ready: bool) -> bool {
        self.asserted(name, ready, None)
    }

    /// Remaining slots include retired scopes that still occupy the bounded
    /// facet table; admission must reserve against physical, not live, slots.
    pub fn remaining_capacity(&self) -> Limits {
        Limits {
            facets: self.limits.facets.saturating_sub(self.scopes.len()),
            assertions: self.limits.assertions.saturating_sub(self.assertions.len()),
            interests: self.limits.interests.saturating_sub(self.interests.len()),
            events: self.limits.events.saturating_sub(self.events.len()),
        }
    }
}
