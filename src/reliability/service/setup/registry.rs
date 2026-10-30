/// Values are an explicitly admitted integer-to-endpoint corpus, identical on all voters.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryAdmission {
    pub object_key: String,
    pub command_schema_ref: String,
    pub endpoints: std::collections::BTreeMap<i64, String>,
}

pub struct Registry {
    admission: RegistryAdmission,
    commands: std::collections::BTreeMap<String, i64>,
    snapshots: std::collections::BTreeMap<String, Option<i64>>,
    current: Option<i64>,
}

impl Registry {
    pub fn new(admission: RegistryAdmission) -> crate::error::Result<Self> {
        if admission.object_key.is_empty()
            || admission.object_key.len() > 256
            || admission.endpoints.is_empty()
            || admission.endpoints.len() > 256
        {
            return Err(crate::error::Failure::invalid_harness(
                "registry requires one bounded key and endpoint corpus",
            ));
        }
        crate::preserves_rail::validate_content_ref(&admission.command_schema_ref)?;
        if admission.endpoints.values().collect::<std::collections::BTreeSet<_>>().len() != admission.endpoints.len() {
            return Err(crate::error::Failure::invalid_harness("registry endpoint corpus must be injective"));
        }
        let entries = admission
            .endpoints
            .iter()
            .map(|(integer, endpoint)| {
                crate::preserves_rail::validate_content_ref(endpoint)?;
                Ok((
                    (command_ref(&admission, *integer)?, *integer),
                    (state_ref(&admission.object_key, Some(endpoint))?, Some(*integer)),
                ))
            })
            .collect::<crate::error::Result<Vec<_>>>()?;
        let (commands, states): (std::collections::BTreeMap<_, _>, Vec<_>) = entries.into_iter().unzip();
        let snapshots = std::iter::once((state_ref(&admission.object_key, None)?, None)).chain(states).collect();
        Ok(Self {
            admission,
            commands,
            snapshots,
            current: None,
        })
    }

    pub fn object_key(&self) -> &str {
        &self.admission.object_key
    }
    pub fn schema_ref(&self) -> &str {
        &self.admission.command_schema_ref
    }
    pub fn values(&self) -> Vec<i64> {
        self.admission.endpoints.keys().copied().collect()
    }
    pub fn value(&self) -> serde_json::Value {
        self.current.map_or_else(
            || serde_json::json!({"type":"null"}),
            |value| serde_json::json!({"type":"integer", "value":value}),
        )
    }
    pub fn command(&self, value: i64) -> crate::error::Result<String> {
        if !self.admission.endpoints.contains_key(&value) {
            return Err(crate::error::Failure::invalid_harness("registry value is outside admitted corpus"));
        }
        command_ref(&self.admission, value)
    }
    pub fn state_ref(&self) -> crate::error::Result<String> {
        state_ref(
            &self.admission.object_key,
            self.current.and_then(|value| self.admission.endpoints.get(&value).map(String::as_str)),
        )
    }
}

impl crate::fabric_consistency::raft::CommittedBatchHandler for Registry {
    fn restore_snapshot(
        &mut self,
        snapshot: &crate::fabric_consistency::raft::SnapshotRestore,
    ) -> crate::error::Result<String> {
        let value = self.snapshots.get(&snapshot.application_state_ref).ok_or_else(|| {
            crate::error::Failure::invalid_harness("registry snapshot is outside admitted state corpus")
        })?;
        self.current = *value;
        self.state_ref()
    }

    fn apply_batch(&mut self, commands: &[crate::fabric_consistency::raft::Command]) -> crate::error::Result<String> {
        if commands.is_empty() {
            return Err(crate::error::Failure::invalid_harness("empty registry commit"));
        }
        let mut next = self.current;
        for command in commands {
            if command.command_schema_ref != self.admission.command_schema_ref {
                return Err(crate::error::Failure::invalid_harness("registry command schema mismatch"));
            }
            next = Some(
                *self
                    .commands
                    .get(&command.command_ref)
                    .ok_or_else(|| crate::error::Failure::invalid_harness("unadmitted registry command"))?,
            );
        }
        self.current = next;
        self.state_ref()
    }
}

fn command_ref(admission: &RegistryAdmission, integer: i64) -> crate::error::Result<String> {
    let endpoint = admission
        .endpoints
        .get(&integer)
        .ok_or_else(|| crate::error::Failure::invalid_harness("unadmitted registry value"))?;
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("live-registry-write-v1", vec![
        crate::preserves_rail::string(&admission.object_key),
        crate::preserves_rail::string(endpoint),
        crate::preserves_rail::string(&admission.command_schema_ref),
    ]))
}

fn state_ref(key: &str, endpoint: Option<&str>) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("live-registry-state-v1", vec![
        crate::preserves_rail::string(key),
        endpoint.map_or_else(|| crate::preserves_rail::record("empty", Vec::new()), crate::preserves_rail::string),
    ]))
}
