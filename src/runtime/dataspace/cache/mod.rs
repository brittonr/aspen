//! Bounded shell for dataspace-access memoization.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Hit,
    Loaded,
}

#[derive(Debug)]
pub struct Access<Value> {
    pub value: std::sync::Arc<Value>,
    pub source: Source,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateError {
    Poisoned,
    Core(molten_core::fabric_durability::cache::Issue),
    EntryCountRepresentation,
    MissingEvictionEntry,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LookupError<LoadError> {
    Projection(molten_core::fabric_durability::cache::Issue),
    State(StateError),
    Load(LoadError),
}

struct Entry<Value> {
    value: std::sync::Arc<Value>,
    accesses_since_promotion: u32,
}

struct State<Value> {
    entries: std::collections::BTreeMap<String, Entry<Value>>,
    eviction_order: std::collections::VecDeque<String>,
}

impl<Value> State<Value> {
    fn empty() -> Self {
        Self {
            entries: std::collections::BTreeMap::new(),
            eviction_order: std::collections::VecDeque::new(),
        }
    }
}

pub struct Store<Value> {
    policy: molten_core::fabric_durability::cache::Policy,
    state: std::sync::Mutex<State<Value>>,
}

impl<Value> Store<Value> {
    // r[impl aspen.dataspace_access_cache.bound]
    // r[impl aspen.dataspace_access_cache.boundary]
    pub fn new(policy: molten_core::fabric_durability::cache::Policy) -> Self {
        Self {
            policy,
            state: std::sync::Mutex::new(State::empty()),
        }
    }

    pub fn policy(&self) -> molten_core::fabric_durability::cache::Policy {
        self.policy
    }

    pub fn len(&self) -> Result<u32, StateError> {
        let state = self.lock()?;
        entry_count(&state)
    }

    pub fn is_empty(&self) -> Result<bool, StateError> {
        self.len().map(|count| count == 0)
    }

    pub fn try_len(&self) -> Option<u32> {
        match self.state.try_lock() {
            Ok(state) => entry_count(&state).ok(),
            Err(std::sync::TryLockError::Poisoned(_)) | Err(std::sync::TryLockError::WouldBlock) => None,
        }
    }

    // r[impl aspen.dataspace_access_cache.deferral]
    // r[impl aspen.dataspace_access_cache.boundary]
    pub fn lookup_or_load<LoadError, Load>(
        &self,
        projection: &molten_core::fabric_durability::cache::AccessProjection<'_>,
        load: Load,
    ) -> Result<Access<Value>, LookupError<LoadError>>
    where
        Load: FnOnce() -> Result<Value, LoadError>,
    {
        let key = molten_core::fabric_durability::cache::project_key(projection).map_err(LookupError::Projection)?;
        if let Some(access) = self.lookup(&key).map_err(LookupError::State)? {
            return Ok(access);
        }

        let candidate = std::sync::Arc::new(load().map_err(LookupError::Load)?);
        let mutation = {
            let mut state = self.lock().map_err(LookupError::State)?;
            insert_or_reuse(&mut state, self.policy, &key, candidate)
        };
        let (access, deferred_releases) = mutation.map_err(LookupError::State)?;
        drop(deferred_releases);
        Ok(access)
    }

    fn lookup(&self, key: &str) -> Result<Option<Access<Value>>, StateError> {
        let mut state = self.lock()?;
        Ok(apply_hit(&mut state, self.policy, key))
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, State<Value>>, StateError> {
        self.state.lock().map_err(|_| StateError::Poisoned)
    }
}

fn apply_hit<Value>(
    state: &mut State<Value>,
    policy: molten_core::fabric_durability::cache::Policy,
    key: &str,
) -> Option<Access<Value>> {
    let (value, promotion) = {
        let entry = state.entries.get_mut(key)?;
        entry.accesses_since_promotion = entry.accesses_since_promotion.saturating_add(1);
        let promotion = molten_core::fabric_durability::cache::decide_promotion(policy, entry.accesses_since_promotion);
        if promotion == molten_core::fabric_durability::cache::Promotion::Promote {
            entry.accesses_since_promotion = 0;
        }
        (std::sync::Arc::clone(&entry.value), promotion)
    };
    if promotion == molten_core::fabric_durability::cache::Promotion::Promote {
        state.eviction_order.retain(|candidate| candidate != key);
        state.eviction_order.push_back(key.to_string());
    }
    Some(Access {
        value,
        source: Source::Hit,
    })
}

fn insert_or_reuse<Value>(
    state: &mut State<Value>,
    policy: molten_core::fabric_durability::cache::Policy,
    key: &str,
    candidate: std::sync::Arc<Value>,
) -> Result<(Access<Value>, Vec<std::sync::Arc<Value>>), StateError> {
    if let Some(access) = apply_hit(state, policy, key) {
        return Ok((access, vec![candidate]));
    }

    let active_count = entry_count(state)?;
    let eviction_order = state.eviction_order.iter().cloned().collect::<Vec<_>>();
    let plan =
        molten_core::fabric_durability::cache::plan_insertion(&molten_core::fabric_durability::cache::InsertionInput {
            policy,
            active_count,
            eviction_order: &eviction_order,
        })
        .map_err(StateError::Core)?;
    let mut deferred_releases = Vec::with_capacity(plan.evict_keys.len());
    for evict_key in &plan.evict_keys {
        let Some(entry) = state.entries.remove(evict_key) else {
            return Err(StateError::MissingEvictionEntry);
        };
        state.eviction_order.retain(|candidate| candidate != evict_key);
        deferred_releases.push(entry.value);
    }

    state.entries.insert(key.to_string(), Entry {
        value: std::sync::Arc::clone(&candidate),
        accesses_since_promotion: 0,
    });
    state.eviction_order.push_back(key.to_string());
    let actual_count = entry_count(state)?;
    if actual_count != plan.active_after_insert || actual_count > policy.capacity {
        return Err(StateError::Core(molten_core::fabric_durability::cache::Issue::ActiveCountExceedsCapacity));
    }

    Ok((
        Access {
            value: candidate,
            source: Source::Loaded,
        },
        deferred_releases,
    ))
}

fn entry_count<Value>(state: &State<Value>) -> Result<u32, StateError> {
    u32::try_from(state.entries.len()).map_err(|_| StateError::EntryCountRepresentation)
}

#[cfg(test)]
mod tests;
