// Durable receiver-owned recording store transitions.
impl RecordedSessionStore {
    /// The receiver root authenticates this LOCAL recording owner. Its initialized
    /// identity is read from the root, never accepted as an admission receipt.
    pub fn open(receiver_root: &crate::node_state::DirectoryRoot) -> Result<Self> {
        use redb::ReadableDatabase;
        let identity = recorded_identity(receiver_root)?;
        let storage = receiver_root.control_service()?.open_subdir(
            &crate::node_state::RelativePath::parse("remote-recorded-sessions-v1")?,
        )?;
        let path = crate::node_state::RelativePath::parse("sessions.redb")?;
        let has_existing_file = storage.try_exists(&path)?;
        let file = storage.open_database_file(&path)?;
        let is_initialized = file.metadata().map_err(recorded_error)?.len() > 0;
        if has_existing_file && !is_initialized {
            return Err(recorded_error("existing recording database is empty; generation reconciliation required"));
        }
        let mut builder = redb::Database::builder();
        if is_initialized {
            // Repair must not silently roll a session generation or close back.
            builder.set_repair_callback(|session| session.abort());
        }
        let database = builder.create_file(file).map_err(recorded_error)?;
        if is_initialized {
            let read = database.begin_read().map_err(recorded_error)?;
            read.open_table(RECORDED_SESSIONS).map_err(recorded_error)?;
        } else {
            let mut write = database.begin_write().map_err(recorded_error)?;
            write.set_durability(redb::Durability::Immediate).map_err(recorded_error)?;
            { write.open_table(RECORDED_SESSIONS).map_err(recorded_error)?; }
            write.commit().map_err(recorded_error)?;
        }
        Ok(Self { database, receiver: identity.node_id, receiver_identity: identity.identity_ref,
            receiver_root: receiver_root.clone() })
    }

    /// Possession of BOTH initialized roots admits a local recorded session only.
    /// It grants no network peer, delivery, capability, or workload authority.
    pub fn admit_local(
        &self,
        sender_root: &crate::node_state::DirectoryRoot,
        topic: &str,
    ) -> Result<RecordedSessionCommit> {
        use redb::ReadableTable;
        validate_name(topic, "recorded session topic")?;
        if recorded_identity(&self.receiver_root)?.identity_ref != self.receiver_identity {
            return Err(recorded_error("receiver identity changed; reopen owner before readmission"));
        }
        let sender = recorded_identity(sender_root)?;
        let key = recorded_key(&sender.node_id, &self.receiver, topic)?;
        let mut write = self.database.begin_write().map_err(recorded_error)?;
        write.set_durability(redb::Durability::Immediate).map_err(recorded_error)?;
        let result;
        {
            let mut table = write.open_table(RECORDED_SESSIONS).map_err(recorded_error)?;
            let previous = table.get(key.as_str()).map_err(recorded_error)?
                .map(|entry| decode_recorded(entry.value())).transpose()?;
            if previous.is_none() && redb::ReadableTableMetadata::len(&table).map_err(recorded_error)? >= MAX_REMOTE_SESSIONS as u64 {
                return Err(recorded_error("session count bound exceeded"));
            }
            if let Some(prior) = &previous {
                if recorded_key(&prior.sender, &prior.receiver, &prior.topic)? != key {
                    return Err(recorded_error("recorded key binding mismatch"));
                }
                replay_recorded(prior)?;
                if !prior.closed && prior.sender_identity == sender.identity_ref && prior.receiver_identity == self.receiver_identity {
                    return Err(recorded_error("close the current generation before readmission"));
                }
            }
            let generation = previous.as_ref().map_or(Some(1), |prior| prior.generation.checked_add(1))
                .ok_or_else(|| recorded_error("generation exhausted"))?;
            let prior_head = previous.as_ref().and_then(|prior| prior.observations.last())
                .map(|observation| string(&observation.observation_ref)).into_iter().collect();
            let session_ref = canonical_hash(&record("remote-recorded-session-binding-v1", vec![
                string(&sender.node_id), string(&self.receiver), string(topic), u64_value(generation),
                string(&sender.identity_ref), string(&self.receiver_identity),
            ]))?;
            let mut data = RecordedSessionData {
                sender: sender.node_id, receiver: self.receiver.clone(), topic: topic.to_owned(), generation,
                session_ref, sender_identity: sender.identity_ref, receiver_identity: self.receiver_identity.clone(),
                closed: false, observations: Vec::new(),
            };
            result = append_recorded(&mut data, "open", &record("local-capability-bootstrap-v1", vec![sequence(prior_head)]), &[], None)?;
            table.insert(key.as_str(), encode_recorded(&data)?.as_slice()).map_err(recorded_error)?;
        }
        write.commit().map_err(recorded_error)?;
        Ok(result)
    }

    pub fn sender_view(&self, sender_root: &crate::node_state::DirectoryRoot, topic: &str, generation: u64) -> Result<SenderSessionView<'_>> {
        let sender = recorded_identity(sender_root)?;
        let view = SenderSessionView { store: self, key: recorded_key(&sender.node_id, &self.receiver, topic)?, generation,
            sender_root: sender_root.clone() };
        Ok(view)
    }

    /// Apply an operation to the recorded receiver dataspace and commit its
    /// observation atomically. No imported receipt can introduce an assertion.
    pub fn commit(
        &self, sender: &str, topic: &str, generation: u64, operation: Operation, payload: &IoValue,
    ) -> Result<RecordedSessionCommit> {
        require_recorded_commit(self.commit_with_evidence(sender, topic, generation, operation, payload)?)
    }

    pub fn commit_with_evidence(
        &self, sender: &str, topic: &str, generation: u64, operation: Operation, payload: &IoValue,
    ) -> Result<RecordedSessionOutcome> {
        self.transition(RecordedTransitionInput { sender, topic, generation, operation: operation.as_str(), payload,
            content_refs: &[], source_envelope_ref: None })
    }

    pub fn close(&self, sender: &str, topic: &str, generation: u64) -> Result<RecordedSessionCommit> {
        require_recorded_commit(self.transition(RecordedTransitionInput { sender, topic, generation, operation: "close",
            payload: &record("session-close-v1", vec![]), content_refs: &[], source_envelope_ref: None })?)
    }

    pub fn receive(&self, generation: u64, envelope: &Envelope) -> Result<RecordedSessionCommit> {
        require_recorded_commit(self.receive_with_evidence(generation, envelope)?)
    }

    pub fn receive_with_evidence(&self, generation: u64, envelope: &Envelope) -> Result<RecordedSessionOutcome> {
        match parse_envelope(&envelope.value) {
            Ok(parsed) if parsed == *envelope => {}
            Ok(parsed) => return recorded_denial(&self.envelope_denial(&parsed, generation, &parsed.envelope_ref),
                "envelope fields differ from committed wire value".to_owned()),
            Err(error) => {
                let source_ref = canonical_hash(&envelope.value)?;
                return recorded_denial(&self.envelope_denial(envelope, generation, &source_ref), error.to_string());
            }
        }
        if envelope.to_peer != self.receiver {
            return recorded_denial(&self.envelope_denial(envelope, generation, &envelope.envelope_ref),
                "receiver peer mismatch".to_owned());
        }
        self.transition(RecordedTransitionInput { sender: &envelope.from_peer, topic: &envelope.topic, generation,
            operation: envelope.operation.as_str(), payload: &envelope.payload, content_refs: &envelope.content_refs,
            source_envelope_ref: Some(&envelope.envelope_ref) })
    }

    fn envelope_denial<'a>(
        &'a self, envelope: &'a Envelope, generation: u64, source_envelope_ref: &'a str,
    ) -> RecordedDenialInput<'a> {
        RecordedDenialInput { sender: &envelope.from_peer, receiver: &self.receiver, topic: &envelope.topic, generation,
            payload: &envelope.payload, source_envelope_ref: Some(source_envelope_ref) }
    }

    fn transition(&self, input: RecordedTransitionInput<'_>) -> Result<RecordedSessionOutcome> {
        use redb::ReadableTable;
        let RecordedTransitionInput { sender, topic, generation, operation, payload, content_refs, source_envelope_ref } = input;
        let denial =
            RecordedDenialInput { sender, receiver: &self.receiver, topic, generation, payload, source_envelope_ref };
        let key = recorded_key(sender, &self.receiver, topic)?;
        let mut write = self.database.begin_write().map_err(recorded_error)?;
        write.set_durability(redb::Durability::Immediate).map_err(recorded_error)?;
        let result;
        {
            let mut table = write.open_table(RECORDED_SESSIONS).map_err(recorded_error)?;
            let mut data = {
                let Some(entry) = table.get(key.as_str()).map_err(recorded_error)? else {
                    return recorded_denial(&denial, "unknown session".to_owned());
                };
                match decode_recorded(entry.value()) {
                    Ok(data) => data,
                    Err(error) => return recorded_denial(&denial, error.to_string()),
                }
            };
            if recorded_key(&data.sender, &data.receiver, &data.topic)? != key {
                return recorded_denial(&denial, "recorded key binding mismatch".to_owned());
            }
            if recorded_identity(&self.receiver_root)?.identity_ref != self.receiver_identity {
                return recorded_denial(&denial, "receiver identity changed; readmission required".to_owned());
            }
            if data.generation != generation || data.closed || data.receiver_identity != self.receiver_identity {
                return recorded_denial(&denial, "stale, closed, or identity-replaced session".to_owned());
            }
            result = match append_recorded(&mut data, operation, payload, content_refs, source_envelope_ref) {
                Ok(commit) => commit,
                Err(error) => return recorded_denial(&denial, error.to_string()),
            };
            let bytes = match encode_recorded(&data) {
                Ok(bytes) => bytes,
                Err(error) => return recorded_denial(&denial, error.to_string()),
            };
            table.insert(key.as_str(), bytes.as_slice()).map_err(recorded_error)?;
        }
        write.commit().map_err(recorded_error)?;
        Ok(RecordedSessionOutcome::Committed(result))
    }
}
