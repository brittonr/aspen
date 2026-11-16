#[cfg(test)]
mod tests;

use crate::system_extension::NativeHostJournal;

const MAX_VALUE_BYTES: u64 = 262_144;
const CALLBACK_RECEIPT_FIELDS: u64 = 16;

/// Observed actual native state, not a caller-authored assertion of success.
/// The original canonical successful callback receipt is materialized through
/// the native value owner and anchored by its existing durable operation journal.
#[derive(Clone, Debug)]
pub struct Commit {
    instance_id: String,
    ingress_request_ref: String,
    content_ref: String,
    callback_receipt_ref: String,
    event_ref: String,
    generation: u64,
    sequence: u64,
    state: crate::system_extension::NativeCallbackValue,
    journal_commit_ref: String,
}

struct Receipt {
    manifest_ref: String,
    generation: u64,
    sequence: u64,
    event_ref: String,
    state_ref: String,
}

impl Commit {
    pub fn observe<P: crate::fabric_execution::ExecutionFabricPort>(
        host: &crate::system_extension::Dispatcher<
            crate::system_extension::Process<P, crate::system_extension::Durable>,
        >,
        ingress_request_ref: &str,
    ) -> crate::error::Result<Self> {
        Self::observe_optional(host, ingress_request_ref)?
            .ok_or_else(|| super::invalid("native actor ingress has no completed durable operation"))
    }

    /// Known absence is pending/unknown; malformed durable evidence remains an error.
    pub fn observe_optional<P: crate::fabric_execution::ExecutionFabricPort>(
        host: &crate::system_extension::Dispatcher<
            crate::system_extension::Process<P, crate::system_extension::Durable>,
        >,
        ingress_request_ref: &str,
    ) -> crate::error::Result<Option<Self>> {
        let instance_id = host
            .executor()
            .instance()
            .lock()
            .map_err(|_| super::invalid("native actor instance lock poisoned"))?
            .instance_id
            .clone();
        let journal =
            host.executor().journal().lock().map_err(|_| super::invalid("native actor journal lock poisoned"))?;
        let history = journal.history(&instance_id).map_err(super::problem)?;
        let ingress = history.iter().rev().find_map(|record| {
            record.completed_operations.iter().find(|operation| {
                operation.operation_ref == ingress_request_ref
                    && operation.kind == crate::system_extension::NativeOperationKind::Ingress
                    && operation.generation == host.state().generation
                    && operation.state == crate::system_extension::NativeOperationState::Terminal
            })
        });
        let Some(ingress) = ingress else {
            return Ok(None);
        };
        let callback_receipt_ref = ingress
            .terminal_ref
            .as_deref()
            .ok_or_else(|| super::invalid("native actor ingress terminal receipt is absent"))?;
        let mut values = host
            .executor()
            .values()
            .lock()
            .map_err(|_| super::invalid("native actor value port lock poisoned"))?;
        let receipt_value = values.materialize(callback_receipt_ref, MAX_VALUE_BYTES).map_err(super::problem)?;
        crate::system_extension::admit_callback_value(&receipt_value, MAX_VALUE_BYTES).map_err(super::problem)?;
        let receipt = read_receipt(&receipt_value)?;
        if receipt.manifest_ref != host.manifest().manifest_ref() || receipt.generation != host.state().generation.get()
        {
            return Err(super::invalid(
                "native actor original callback receipt belongs to another manifest or generation",
            ));
        }
        let state = values.materialize(&receipt.state_ref, MAX_VALUE_BYTES).map_err(super::problem)?;
        crate::system_extension::admit_callback_value(&state, MAX_VALUE_BYTES).map_err(super::problem)?;
        let mut proof = Self {
            instance_id,
            ingress_request_ref: ingress_request_ref.to_string(),
            content_ref: ingress.parent_ref.clone(),
            callback_receipt_ref: callback_receipt_ref.to_string(),
            event_ref: receipt.event_ref,
            generation: receipt.generation,
            sequence: receipt.sequence,
            state,
            journal_commit_ref: String::new(),
        };
        // Select the original semantic commit, not the latest state after later callbacks.
        // The selected reference therefore remains stable across reopening and subsequent work.
        for record in &history {
            if proof.validate_record(record, &ingress.parent_ref, receipt.generation).is_ok() {
                proof.journal_commit_ref = crate::system_extension::canonical_instance_record(record)?.record_ref;
                return Ok(Some(proof));
            }
        }
        Err(super::invalid("native actor callback has no exact durable semantic commit record"))
    }

    pub fn state(&self) -> &crate::system_extension::NativeCallbackValue {
        &self.state
    }
    pub fn event_ref(&self) -> &str {
        &self.event_ref
    }
    pub fn commit_ref(&self) -> &str {
        &self.journal_commit_ref
    }
    pub fn callback_receipt_ref(&self) -> &str {
        &self.callback_receipt_ref
    }
    pub fn ingress_request_ref(&self) -> &str {
        &self.ingress_request_ref
    }

    /// Exact payload identity from the original completed native ingress journal row.
    pub fn content_ref(&self) -> &str {
        &self.content_ref
    }

    pub(super) fn binding_value(&self) -> preserves::IOValue {
        crate::preserves_rail::record("actor-native-semantic-commit-v1", vec![
            crate::preserves_rail::string(&self.instance_id),
            crate::preserves_rail::string(&self.ingress_request_ref),
            crate::preserves_rail::string(self.content_ref()),
            crate::preserves_rail::string(self.event_ref()),
            crate::preserves_rail::string(self.callback_receipt_ref()),
            crate::preserves_rail::string(&self.state.value_ref),
            crate::preserves_rail::string(self.commit_ref()),
        ])
    }

    pub(super) fn validate<P: crate::fabric_execution::ExecutionFabricPort>(
        &self,
        host: &crate::system_extension::Dispatcher<
            crate::system_extension::Process<P, crate::system_extension::Durable>,
        >,
        content_ref: &str,
        generation: u64,
    ) -> crate::error::Result<()> {
        {
            let instance = host
                .executor()
                .instance()
                .lock()
                .map_err(|_| super::invalid("native actor instance lock poisoned"))?;
            if instance.instance_id != self.instance_id
                || instance.manifest_ref != host.manifest().manifest_ref()
                || instance.lifecycle.generation.get() != generation
                || host.state().generation.get() != generation
            {
                return Err(super::invalid("native actor semantic commit belongs to another admitted instance"));
            }
        }
        let journal =
            host.executor().journal().lock().map_err(|_| super::invalid("native actor journal lock poisoned"))?;
        for record in &journal.history(&self.instance_id).map_err(super::problem)? {
            if crate::system_extension::canonical_instance_record(record)?.record_ref == self.journal_commit_ref {
                if record.manifest_ref != host.manifest().manifest_ref() {
                    return Err(super::invalid("native actor semantic journal manifest mismatch"));
                }
                return self.validate_record(record, content_ref, generation);
            }
        }
        Err(super::invalid("native actor semantic commit is not in the durable native journal"))
    }

    fn validate_record(
        &self,
        record: &crate::system_extension::NativeInstanceRecord,
        content_ref: &str,
        generation: u64,
    ) -> crate::error::Result<()> {
        if record.instance_id != self.instance_id
            || record.lifecycle.generation.get() != generation
            || self.generation != generation
            || record.state_ref.as_deref() != Some(self.state.value_ref.as_str())
            || self.content_ref != content_ref
        {
            return Err(super::invalid(
                "native actor semantic completion does not match exact instance, generation and state",
            ));
        }
        let callback_operation_ref = crate::system_extension::native_identity_ref(&[
            "native-callback-operation-v2",
            &record.instance_id,
            &record.manifest_ref,
            "request",
            &generation.to_string(),
            &self.sequence.to_string(),
            &self.event_ref,
        ]);
        let state_publication_ref =
            publication_ref(&callback_operation_ref, "state", &self.state.value_ref, generation);
        let receipt_publication_ref = publication_ref(
            &self.ingress_request_ref,
            "ingress-callback-receipt",
            &self.callback_receipt_ref,
            generation,
        );
        let is_ingress_completed = record.completed_operations.iter().any(|operation| {
            completed(operation, crate::system_extension::NativeOperationKind::Ingress, generation)
                && operation.operation_ref == self.ingress_request_ref
                && operation.parent_ref == content_ref
                && operation.terminal_ref.as_deref() == Some(self.callback_receipt_ref())
        });
        let is_callback_completed = record.completed_operations.iter().any(|operation| {
            completed(operation, crate::system_extension::NativeOperationKind::Callback, generation)
                && operation.operation_ref == callback_operation_ref
                && operation.parent_ref == self.event_ref
        });
        let is_state_published =
            publication_completed(record, &state_publication_ref, &callback_operation_ref, generation);
        let is_receipt_published =
            publication_completed(record, &receipt_publication_ref, &self.ingress_request_ref, generation);
        if !is_ingress_completed || !is_callback_completed || !is_state_published || !is_receipt_published {
            return Err(super::invalid(
                "native actor completion lacks exact original ingress, callback, state or receipt publication",
            ));
        }
        Ok(())
    }
}

include!("native/parts/codec/body.rs");
