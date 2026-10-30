use super::*;

const STATE_KEY: &[u8] = b"collaboration/current";

/// Uses the existing capability-rooted, admitted Redb durability adapter. No
/// ambient path, second database implementation, or visibility-only write.
pub struct Database {
    adapter: super::storage::Backend,
    domain: crate::fabric_durability::AtomicityDomain,
}

impl Database {
    pub fn new(
        adapter: crate::fabric_durability::RedbDurableStateAdapter,
        domain: crate::fabric_durability::AtomicityDomain,
    ) -> crate::error::Result<Self> {
        Self::from_storage(super::storage::Backend::Live(Box::new(adapter)), domain)
    }

    pub fn simulated(
        adapter: crate::fabric_durability::SimulatedDurableStateAdapter,
        domain: crate::fabric_durability::AtomicityDomain,
    ) -> crate::error::Result<Self> {
        Self::from_storage(
            super::storage::Backend::Simulation {
                adapter: Box::new(adapter),
                next_fault: None,
                is_uncertain: false,
            },
            domain,
        )
    }

    fn from_storage(
        adapter: super::storage::Backend,
        domain: crate::fabric_durability::AtomicityDomain,
    ) -> crate::error::Result<Self> {
        let descriptor = &adapter.state().descriptor;
        if adapter.health() != crate::fabric_durability::AdapterHealth::Serving
            || domain.adapter_id != descriptor.adapter_id
            || domain.namespace_id != descriptor.namespace_id
            || domain.generation != descriptor.generation
            || domain != descriptor.atomicity_domain
            || !domain.object_classes.contains(&crate::fabric_durability::DurableObjectClass::OrderedValue)
            || !domain.supported_levels.contains(&crate::fabric_durability::DurabilityLevel::MachineLoss)
        {
            return Err(denied(Denial::Stale));
        }
        Ok(Self { adapter, domain })
    }

    pub fn profile_ref(&self) -> &str {
        self.adapter.profile_ref()
    }
    pub fn mode(&self) -> &str {
        self.adapter.mode()
    }
    pub fn inject_simulated_fault(
        &mut self,
        fault: crate::fabric_durability::SimulatedFault,
    ) -> crate::error::Result<()> {
        self.adapter.inject(fault)
    }
    pub(crate) fn ensure_serving(&self) -> crate::error::Result<()> {
        self.check_health()
    }

    pub fn load(&self) -> crate::error::Result<Option<State>> {
        self.check_health()?;
        self.adapter
            .state()
            .ordered
            .get(STATE_KEY)
            .map(|value| {
                let value = decode_value(&value.value, &value.value_ref)?;
                parse_state(&value)
            })
            .transpose()
    }

    pub fn initialize(&mut self, state: &State) -> crate::error::Result<()> {
        validated_state_ref(state)?;
        if self.load()?.is_some() {
            return Err(denied(Denial::Conflict));
        }
        let value = state_value(state);
        self.put(STATE_KEY.to_vec(), &value, crate::fabric_durability::ValuePrecondition::Missing)
            .map(|_| ())
    }

    pub fn commit(
        &mut self,
        before: &StateRef,
        state: &State,
        receipt: &preserves::IOValue,
    ) -> crate::error::Result<String> {
        self.check_health()?;
        validated_state_ref(state)?;
        let state_value = state_value(state);
        let receipt_ref = crate::preserves_rail::canonical_hash(receipt)?;
        let state_bytes = crate::preserves_rail::canonical_bytes(&state_value)?;
        let receipt_bytes = crate::preserves_rail::canonical_bytes(receipt)?;
        if state_bytes.len() as u64 > MAX_RECORD_BYTES || receipt_bytes.len() as u64 > MAX_RECORD_BYTES {
            return Err(denied(Denial::Bound));
        }
        let mut mutations = vec![
            crate::fabric_durability::OrderedMutation::Put {
                key: STATE_KEY.to_vec(),
                value: state_bytes,
                value_ref: crate::preserves_rail::canonical_hash(&state_value)?,
                precondition: crate::fabric_durability::ValuePrecondition::ValueRef(before.as_str().to_string()),
            },
            crate::fabric_durability::OrderedMutation::Put {
                key: format!("collaboration/receipt/{receipt_ref}").into_bytes(),
                value: receipt_bytes,
                value_ref: receipt_ref.clone(),
                precondition: crate::fabric_durability::ValuePrecondition::Missing,
            },
        ];
        mutations.extend(self.result_mutations(receipt, &receipt_ref)?);
        let request = crate::fabric_durability::AtomicBatchRequest {
            domain: self.domain.clone(),
            generation: self.domain.generation,
            mutations,
            durability: crate::fabric_durability::DurabilityLevel::MachineLoss,
        };
        let outcome = self.adapter.apply_batch(&request)?;
        if outcome.outcome != crate::fabric_durability::MutationOutcome::Durable {
            return Err(denied(Denial::Uncertain));
        }
        Ok(receipt_ref)
    }

    fn result_mutations(
        &self,
        receipt: &preserves::IOValue,
        receipt_ref: &str,
    ) -> crate::error::Result<Vec<crate::fabric_durability::OrderedMutation>> {
        let fields = decode::fields(receipt, "collaboration-commit-receipt-v1", 7)?;
        let mut requests = std::collections::BTreeSet::new();
        for value in decode::items(&fields[4], MAX_OPERATIONS)? {
            requests.insert(decode::string(&value)?);
        }
        let value =
            crate::preserves_rail::record("collaboration-operation-result-v1", vec![crate::preserves_rail::string(
                receipt_ref,
            )]);
        let bytes = crate::preserves_rail::canonical_bytes(&value)?;
        let reference = crate::preserves_rail::canonical_hash(&value)?;
        // At most one result mutation per decoded request, already bounded by MAX_OPERATIONS.
        Ok(requests
            .into_iter()
            .map(|request| format!("collaboration/result/{request}").into_bytes())
            .filter(|key| !self.adapter.state().ordered.contains_key(key))
            .map(|key| crate::fabric_durability::OrderedMutation::Put {
                key,
                value: bytes.clone(),
                value_ref: reference.clone(),
                precondition: crate::fabric_durability::ValuePrecondition::Missing,
            })
            .collect())
    }

    pub fn has_result(&self, request: &str) -> crate::error::Result<bool> {
        self.check_health()?;
        Ok(self.adapter.state().ordered.contains_key(format!("collaboration/result/{request}").as_bytes()))
    }

    pub fn result(&self, request: &str) -> crate::error::Result<String> {
        self.check_health()?;
        let key = format!("collaboration/result/{request}").into_bytes();
        let value = self.adapter.state().ordered.get(&key).ok_or_else(|| denied(Denial::Missing))?;
        let value = decode_value(&value.value, &value.value_ref)?;
        let fields = decode::fields(&value, "collaboration-operation-result-v1", 1)?;
        decode::string(&fields[0])
    }

    pub(crate) fn receipt(&self, reference: &str) -> crate::error::Result<preserves::IOValue> {
        self.check_health()?;
        crate::preserves_rail::validate_content_ref(reference)?;
        let key = format!("collaboration/receipt/{reference}").into_bytes();
        let value = self.adapter.state().ordered.get(&key).ok_or_else(|| denied(Denial::Missing))?;
        if value.value_ref != reference {
            return Err(denied(Denial::Domain));
        }
        decode_value(&value.value, &value.value_ref)
    }

    pub fn stage(&mut self, value: &preserves::IOValue) -> crate::error::Result<String> {
        let reference = crate::preserves_rail::canonical_hash(value)?;
        let key = format!("collaboration/command/{reference}").into_bytes();
        self.check_health()?;
        if let Some(existing) = self.adapter.state().ordered.get(&key) {
            if existing.value_ref != reference {
                return Err(denied(Denial::Conflict));
            }
            return Ok(reference);
        }
        if self.adapter.state().ordered.len() > MAX_OPERATIONS * 3 {
            return Err(denied(Denial::Bound));
        }
        self.put(key, value, crate::fabric_durability::ValuePrecondition::Missing)?;
        Ok(reference)
    }

    pub fn has_command(&self, reference: &str) -> crate::error::Result<bool> {
        self.check_health()?;
        Ok(self.adapter.state().ordered.contains_key(format!("collaboration/command/{reference}").as_bytes()))
    }

    pub fn command(&self, reference: &str) -> crate::error::Result<preserves::IOValue> {
        self.check_health()?;
        let key = format!("collaboration/command/{reference}").into_bytes();
        let value = self.adapter.state().ordered.get(&key).ok_or_else(|| denied(Denial::Missing))?;
        decode_value(&value.value, reference)
    }

    pub(crate) fn reserve_replay(
        &mut self,
        key_ref: &str,
        digest: &str,
    ) -> Result<(), basalt::ucan::HolderInvocationReplayError> {
        self.check_health().map_err(|_| basalt::ucan::HolderInvocationReplayError::Backend)?;
        let key = format!("collaboration/replay/{key_ref}").into_bytes();
        if self.adapter.state().ordered.contains_key(&key) {
            return Err(basalt::ucan::HolderInvocationReplayError::Duplicate);
        }
        if self.adapter.state().ordered.len() >= MAX_OPERATIONS {
            return Err(basalt::ucan::HolderInvocationReplayError::Denied);
        }
        let value = crate::preserves_rail::record("collaboration-replay-admission-v1", vec![
            crate::preserves_rail::string(key_ref),
            crate::preserves_rail::string(digest),
        ]);
        self.put(key, &value, crate::fabric_durability::ValuePrecondition::Missing)
            .map(|_| ())
            .map_err(|_| basalt::ucan::HolderInvocationReplayError::Backend)
    }

    fn put(
        &mut self,
        key: Vec<u8>,
        value: &preserves::IOValue,
        precondition: crate::fabric_durability::ValuePrecondition,
    ) -> crate::error::Result<String> {
        self.check_health()?;
        let bytes = crate::preserves_rail::canonical_bytes(value)?;
        if bytes.len() as u64 > MAX_RECORD_BYTES {
            return Err(denied(Denial::Bound));
        }
        let request = crate::fabric_durability::AtomicBatchRequest {
            domain: self.domain.clone(),
            generation: self.domain.generation,
            mutations: vec![crate::fabric_durability::OrderedMutation::Put {
                key,
                value: bytes,
                value_ref: crate::preserves_rail::canonical_hash(value)?,
                precondition,
            }],
            durability: crate::fabric_durability::DurabilityLevel::MachineLoss,
        };
        let outcome = self.adapter.apply_batch(&request)?;
        if outcome.outcome != crate::fabric_durability::MutationOutcome::Durable {
            return Err(denied(Denial::Uncertain));
        }
        Ok(outcome.transition_ref)
    }

    fn check_health(&self) -> crate::error::Result<()> {
        if self.adapter.health() != crate::fabric_durability::AdapterHealth::Serving {
            return Err(denied(Denial::Uncertain));
        }
        Ok(())
    }
}

fn decode_value(bytes: &[u8], reference: &str) -> crate::error::Result<preserves::IOValue> {
    if bytes.len() as u64 > MAX_RECORD_BYTES {
        return Err(denied(Denial::Bound));
    }
    let value = crate::preserves_rail::parse_canonical_bytes(bytes)?;
    if crate::preserves_rail::canonical_hash(&value)? != reference {
        return Err(denied(Denial::Domain));
    }
    Ok(value)
}
