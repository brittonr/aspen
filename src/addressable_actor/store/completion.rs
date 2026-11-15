use redb::ReadableTable;

use super::*;

pub(super) const COMPLETION_TABLE: redb::TableDefinition<&str, &[u8]> =
    redb::TableDefinition::new("addressable_actor_completion_evidence_v1");

/// Existing actor-owner evidence only; no semantic state or authority is reconstructed here.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Evidence {
    pub ingress_ref: String,
    pub actor_key_ref: String,
    pub operation: ActorOperation,
    pub acknowledgement: molten_core::coordination_delivery::DeliveryRequest,
    pub receipt_bytes: Vec<u8>,
    pub receipt_ref: String,
}

impl Evidence {
    pub(crate) fn validate(&self) -> crate::error::Result<()> {
        let decoded = crate::preserves_rail::strict_canonical_decode(&self.receipt_bytes)?;
        let fields = crate::preserves_rail::simple_record_fields(&decoded.value, ACTOR_COMMIT_RECEIPT_RECORD, 21)?;
        let text = |index: usize, label: &str| crate::preserves_rail::record_string_field(&fields[index], label, label);
        let request_ref = text(2, "request-ref")?;
        let ActorOperation::CompleteDelivery {
            delivery_item_ref,
            delivery_token_ref,
            ..
        } = &self.operation
        else {
            return Err(crate::error::Failure::invalid_harness("retained actor evidence is not a completion"));
        };
        let molten_core::coordination_delivery::DeliveryOperation::Acknowledge { token } =
            &self.acknowledgement.operation
        else {
            return Err(crate::error::Failure::invalid_harness(
                "retained actor completion has no exact acknowledgement",
            ));
        };
        let effects_value = crate::preserves_rail::value_to_iovalue(&fields[12]);
        let effects = crate::preserves_rail::simple_record_fields(&effects_value, "effect-observations", 1)?;
        let observations = crate::preserves_rail::required_sequence_field(&effects[0], "completion effects")?;
        if observations.len() != 1 {
            return Err(crate::error::Failure::invalid_harness(
                "retained actor completion has no single acknowledgement effect",
            ));
        }
        let effect_value = crate::preserves_rail::value_to_iovalue(&observations[0]);
        let effect = crate::preserves_rail::simple_record_fields(&effect_value, "effect-observation", 4)?;
        let disposition = crate::preserves_rail::required_string_field(&effect[2], "effect disposition")?;
        let outcome = crate::preserves_rail::optional_content_ref_string(&effect[3], "acknowledgement outcome")?;
        if self.receipt_ref != crate::addressable_actor::records::commit_receipt_ref(&self.receipt_bytes)
            || text(0, "schema")? != ACTOR_COMMIT_RECEIPT_SCHEMA
            || text(1, "actor-key-ref")? != self.actor_key_ref
            || text(3, "operation-ref")? != identify_actor_operation(&request_ref, &self.operation)
            || !matches!(text(8, "status")?.as_str(), "applied" | "already-applied" | "applied-after-reconciliation")
            || text(9, "currentness")? != "linearizable"
            || text(10, "durability")? != "durable"
            || disposition != "succeeded"
            || outcome.is_none()
            || token.item_ref != *delivery_item_ref
            || token.token_ref != *delivery_token_ref
            || token.queue_id != self.acknowledgement.queue_id
            || token.token_ref != molten_core::coordination_delivery::identify_delivery_token(token)
        {
            return Err(crate::error::Failure::invalid_harness("retained actor completion evidence is inconsistent"));
        }
        Ok(())
    }

    pub(crate) fn matches_original(
        &self,
        original: Option<&molten_core::coordination_delivery::DeliveryRequest>,
    ) -> bool {
        original.is_none_or(|request| {
            molten_core::coordination_delivery::identify_delivery_request(request)
                == molten_core::coordination_delivery::identify_delivery_request(&self.acknowledgement)
        })
    }
}

impl Local {
    pub(crate) fn retain_completion(&mut self, evidence: &Evidence) -> PortResult<()> {
        evidence.validate().map_err(codec_error)?;
        let key = key(&evidence.actor_key_ref, &evidence.ingress_ref);
        let bytes = serde_json::to_vec(evidence).map_err(codec_error)?;
        if bytes.len() > MAX_ACTOR_STATE_BYTES {
            return Err(port_error("actor-completion-bound", "actor completion evidence exceeds its byte bound"));
        }
        let write = self.database.begin_write().map_err(redb_error)?;
        {
            let mut table = write.open_table(COMPLETION_TABLE).map_err(redb_error)?;
            if let Some(prior) = table.get(key.as_str()).map_err(redb_error)? {
                if prior.value() != bytes.as_slice() {
                    return Err(port_error(
                        "actor-completion-conflict",
                        "original actor completion evidence cannot be replaced",
                    ));
                }
                return Ok(());
            }
            table.insert(key.as_str(), bytes.as_slice()).map_err(redb_error)?;
        }
        write.commit().map_err(redb_error)
    }

    pub(crate) fn completion(&self, actor_key_ref: &str, ingress_ref: &str) -> PortResult<Option<Evidence>> {
        let read = self.database.begin_read().map_err(redb_error)?;
        let table = read.open_table(COMPLETION_TABLE).map_err(redb_error)?;
        let Some(bytes) = table.get(key(actor_key_ref, ingress_ref).as_str()).map_err(redb_error)? else {
            return Ok(None);
        };
        if bytes.value().len() > MAX_ACTOR_STATE_BYTES {
            return Err(port_error(
                "actor-completion-bound",
                "stored actor completion evidence exceeds its byte bound",
            ));
        }
        let evidence: Evidence = serde_json::from_slice(bytes.value()).map_err(codec_error)?;
        evidence.validate().map_err(codec_error)?;
        if evidence.actor_key_ref != actor_key_ref || evidence.ingress_ref != ingress_ref {
            return Err(port_error(
                "actor-completion-binding",
                "stored actor completion evidence belongs to another request",
            ));
        }
        Ok(Some(evidence))
    }
}

fn key(actor_key_ref: &str, ingress_ref: &str) -> String {
    format!("{actor_key_ref}/{ingress_ref}")
}
