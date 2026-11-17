use super::*;
mod actions;

struct Reservation {
    immediate: usize,
    removals: usize,
}

impl super::Table {
    pub fn new(limits: Limits) -> Result<Self, Error> {
        if limits.facets == 0 || limits.facets > MAX_FACETS {
            return Err(Error::Capacity);
        }
        if limits.assertions == 0 || limits.assertions > MAX_ASSERTIONS {
            return Err(Error::Capacity);
        }
        if limits.interests == 0 || limits.interests > MAX_INTERESTS {
            return Err(Error::Capacity);
        }
        if limits.events == 0 || limits.events > MAX_EVENTS {
            return Err(Error::Capacity);
        }
        Ok(Self {
            limits,
            next: 0,
            scopes: alloc::vec::Vec::with_capacity(limits.facets),
            assertions: alloc::vec::Vec::with_capacity(limits.assertions),
            interests: alloc::vec::Vec::with_capacity(limits.interests),
            events: alloc::vec::Vec::with_capacity(limits.events),
        })
    }

    /// Open a host-granted scope. Descendants are retired with their parent.
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "The host exclusively owns this bounded table; successful facet admission commits one serialized scope."
    )]
    pub fn open(&mut self, parent: Option<Facet>, rights: Rights) -> Result<Facet, Error> {
        if self.scopes.len() >= self.limits.facets {
            return Err(Error::Capacity);
        }
        if let Some(parent) = parent {
            attempt!(self.scope(parent));
        }
        let Some(next) = self.next.checked_add(1) else {
            return Err(Error::Capacity);
        };
        self.next = next;
        let id = Facet(next);
        self.scopes.push(Scope {
            id,
            parent,
            rights,
            live: true,
        });
        Ok(id)
    }

    fn scope(&self, facet: Facet) -> Result<&Scope, Error> {
        let mut index = 0;
        while index < self.scopes.len() {
            if self.scopes[index].id == facet
                && self.scopes[index].live
                && !self.retired_ancestor(facet)
            {
                return Ok(&self.scopes[index]);
            }
            index += 1;
        }
        Err(Error::InvalidFacet)
    }

    fn value<'a>(
        &self,
        facet: Facet,
        name: &'a str,
        ready: bool,
        operation: Operation,
    ) -> Result<ServiceRef<'a>, Error> {
        let rights = attempt!(self.scope(facet)).rights;
        if !permits(rights, operation) {
            return Err(Error::Denied);
        }
        if !valid_name(name) {
            return Err(Error::InvalidName);
        }
        Ok(ServiceRef { name, ready })
    }

    fn matches_count(&self) -> usize {
        let mut count = 0;
        let mut index = 0;
        while index < self.interests.len() {
            let is_visible = {
                let service = &self.interests[index].service;
                self.asserted(&service.name, service.ready, None)
            };
            if is_visible {
                count += 1;
            }
            index += 1;
        }
        count
    }

    fn asserted(&self, name: &str, ready: bool, except: Option<usize>) -> bool {
        let mut index = 0;
        while index < self.assertions.len() {
            let service = &self.assertions[index].service;
            if Some(index) != except && service.name == name && service.ready == ready {
                return true;
            }
            index += 1;
        }
        false
    }

    fn interested(&self, facet: Facet, name: &str, ready: bool) -> bool {
        let mut index = 0;
        while index < self.interests.len() {
            let interest = &self.interests[index];
            if interest.owner == facet
                && interest.service.name == name
                && interest.service.ready == ready
            {
                return true;
            }
            index += 1;
        }
        false
    }

    fn interested_count(&self, name: &str, ready: bool) -> usize {
        let mut count = 0;
        let mut index = 0;
        while index < self.interests.len() {
            if self.interests[index].service.name == name
                && self.interests[index].service.ready == ready
            {
                count += 1;
            }
            index += 1;
        }
        count
    }

    fn owned_index(&self, facet: Facet, name: &str) -> Option<usize> {
        let mut index = 0;
        while index < self.assertions.len() {
            if self.assertions[index].owner == facet && self.assertions[index].service.name == name
            {
                return Some(index);
            }
            index += 1;
        }
        None
    }

    /// Preserve enough unused event slots for all future removals, including
    /// mandatory failure cleanup of every live assertion-interest pair.
    const fn reserve(&self, reservation: Reservation) -> Result<(), Error> {
        let Some(required) = reservation.immediate.checked_add(reservation.removals) else {
            return Err(Error::Capacity);
        };
        if required > self.limits.events.saturating_sub(self.events.len()) {
            Err(Error::Capacity)
        } else {
            Ok(())
        }
    }

    fn descendant(&self, candidate: Facet, parent: Facet) -> bool {
        Self::within(&self.scopes, candidate, parent)
    }

    fn within(scopes: &[Scope], candidate: Facet, parent: Facet) -> bool {
        let mut current = Some(candidate);
        while let Some(id) = current {
            if id == parent {
                return true;
            }
            let mut index = 0;
            let mut next = None;
            while index < scopes.len() {
                if scopes[index].id == id {
                    next = scopes[index].parent;
                    break;
                }
                index += 1;
            }
            current = next;
        }
        false
    }

    fn retired_ancestor(&self, id: Facet) -> bool {
        let mut index = 0;
        let mut parent = None;
        while index < self.scopes.len() {
            if self.scopes[index].id == id {
                parent = self.scopes[index].parent;
                break;
            }
            index += 1;
        }
        while let Some(id) = parent {
            index = 0;
            while index < self.scopes.len() && self.scopes[index].id != id {
                index += 1;
            }
            if index == self.scopes.len() || !self.scopes[index].live {
                return true;
            }
            parent = self.scopes[index].parent;
        }
        false
    }
}
