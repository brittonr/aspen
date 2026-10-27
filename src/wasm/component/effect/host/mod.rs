//! Inactive effect-world host adapter. No engine or linker activation lives here.

pub mod authority;
mod context;
mod recording;

pub use context::CurrentInput;
pub use context::Limits;
pub use context::Scope;
pub use context::limits_ref;
pub use context::recording_ref;
pub use recording::Replay;
pub use recording::ReplayInput;

type Result<T, E = super::super::model::Denial> = std::result::Result<T, E>;

pub struct Session {
    scope: Scope,
    generation: u64,
    incarnation: u64,
    next_sequence: u64,
    is_poisoned: bool,
}

pub struct Outcome {
    pub response_bytes: Vec<u8>,
    pub record: crate::system_extension::NativeCallbackValue,
    pub entry: crate::deterministic_replay::EffectLogEntry,
}

impl Session {
    pub fn new(scope: Scope, generation: u64, incarnation: u64) -> Result<Self> {
        context::validate_scope(&scope)?;
        Ok(Self {
            scope,
            generation,
            incarnation,
            next_sequence: 0,
            is_poisoned: false,
        })
    }

    pub const fn is_poisoned(&self) -> bool {
        self.is_poisoned
    }

    /// Current inputs come from the product composition root, never from the guest.
    /// A host/port failure after reservation poisons this session; it is not a policy denial.
    // r[impl molten.wasm_component_adoption.hostcalls]
    // r[impl molten.wasm_component_adoption.boundary]
    pub fn invoke(
        &mut self,
        request_bytes: &[u8],
        current: &CurrentInput<'_>,
        values: &mut dyn crate::system_extension::NativeCallbackValuePort,
    ) -> Result<Outcome> {
        if self.is_poisoned {
            return Err(super::invalid("component host session requires reconciliation after a host failure"));
        }
        let request = super::decode_host_request(request_bytes)?;
        limits_ref(current.limits)?;
        if request.sequence() != self.next_sequence || self.next_sequence >= current.limits.max_calls {
            return Err(super::invalid("component host request is duplicate, out of order or over budget"));
        }
        let next = self
            .next_sequence
            .checked_add(1)
            .ok_or_else(|| super::invalid("component host sequence overflow"))?;
        let admission = self.admission(&request, current);
        self.next_sequence = next;
        self.is_poisoned = true;
        let outcome = self.dispatch(&request, request_bytes, admission, current.limits, values)?;
        self.is_poisoned = false;
        Ok(outcome)
    }

    fn admission(&self, request: &super::HostRequest, current: &CurrentInput<'_>) -> Result<preserves::IOValue> {
        if current.generation != self.generation || current.incarnation != self.incarnation {
            return Err(super::invalid("component host session lifecycle fence is stale"));
        }
        context::admit(request, &self.scope, current)?;
        authority::admit(request, &self.scope, &current.authority)
    }

    fn dispatch(
        &self,
        request: &super::HostRequest,
        request_bytes: &[u8],
        admission: Result<preserves::IOValue>,
        limits: Limits,
        values: &mut dyn crate::system_extension::NativeCallbackValuePort,
    ) -> Result<Outcome> {
        let is_admitted = admission.is_ok();
        let diagnostic = admission.as_ref().err().map(|denial| denial.canonical_class());
        let admission = admission.unwrap_or_else(|denial| {
            crate::preserves_rail::record("component-host-denied-v1", vec![
                context::scope_value(&self.scope),
                crate::preserves_rail::string(request.request_ref()),
                crate::preserves_rail::string(denial.canonical_class()),
            ])
        });
        let admission = publish_value(values, &admission)?;
        publish_bytes(values, request.request_ref(), request_bytes)?;
        let intent = crate::preserves_rail::record("component-host-intent-v1", vec![
            context::scope_value(&self.scope),
            crate::preserves_rail::string(request.request_ref()),
            crate::preserves_rail::string(&admission.value_ref),
            crate::preserves_rail::string(&request.grant().recorded_effect_ref),
        ]);
        let intent = publish_value(values, &intent)?;
        let output = if is_admitted {
            Some(materialize_blob(values, request, limits.max_output_bytes)?)
        } else {
            None
        };
        let response = crate::effects::effect_response_value(&crate::effects::EffectResponseInput {
            request_ref: request.effect().request_ref.clone(),
            decision: if is_admitted { "pass" } else { "deny" }.to_string(),
            output_ref: output
                .as_ref()
                .map(crate::preserves_rail::canonical_hash)
                .transpose()
                .map_err(super::convert)?,
            diagnostics: diagnostic.into_iter().map(str::to_string).collect(),
            evidence_refs: vec![admission.value_ref, intent.value_ref.clone()],
        })
        .map_err(super::convert)?;
        let response_bytes = super::encode_host_response(super::HostResponseInput {
            request,
            response: &response,
            output: output.as_ref(),
        })?;
        let response_ref = crate::preserves_rail::content_ref_from_bytes(&response_bytes);
        let response = crate::system_extension::NativeCallbackValue {
            value_ref: response_ref.clone(),
            bytes: response_bytes,
        };
        publish(values, &response)?;
        let record = recording::completion(self, request, &response_ref, &intent.value_ref)?;
        let record = publish_value(values, &record)?;
        let entry = recording::entry(&self.scope, request, response_ref);
        Ok(Outcome {
            response_bytes: response.bytes,
            record,
            entry,
        })
    }
}

fn materialize_blob(
    values: &mut dyn crate::system_extension::NativeCallbackValuePort,
    request: &super::HostRequest,
    maximum: u64,
) -> Result<preserves::IOValue> {
    let resource = authority::blob_resource(request)?;
    let object_ref = resource
        .strip_prefix(authority::BLOB_RESOURCE_PREFIX)
        .ok_or_else(|| super::invalid("component host blob resource mapping is invalid"))?;
    let value = values
        .materialize(object_ref, maximum)
        .map_err(|_| super::invalid("component host blob materialization failed after admission"))?;
    crate::system_extension::admit_callback_value(&value, maximum)
        .map_err(|_| super::invalid("component host blob materialization exceeded its bound or identity"))?;
    if value.value_ref != object_ref {
        return Err(super::invalid("component host blob port returned another object"));
    }
    super::bounded(&value.bytes)?;
    crate::preserves_rail::parse_canonical_bytes(&value.bytes).map_err(super::convert)
}

fn publish_value(
    values: &mut dyn crate::system_extension::NativeCallbackValuePort,
    value: &preserves::IOValue,
) -> Result<crate::system_extension::NativeCallbackValue> {
    let bytes = crate::preserves_rail::canonical_bytes(value).map_err(super::convert)?;
    let value_ref = crate::preserves_rail::content_ref_from_bytes(&bytes);
    let value = crate::system_extension::NativeCallbackValue { value_ref, bytes };
    publish(values, &value)?;
    Ok(value)
}

fn publish_bytes(
    values: &mut dyn crate::system_extension::NativeCallbackValuePort,
    value_ref: &str,
    bytes: &[u8],
) -> Result<()> {
    publish(values, &crate::system_extension::NativeCallbackValue {
        value_ref: value_ref.to_string(),
        bytes: bytes.to_vec(),
    })
}

fn publish(
    values: &mut dyn crate::system_extension::NativeCallbackValuePort,
    value: &crate::system_extension::NativeCallbackValue,
) -> Result<()> {
    super::bounded(&value.bytes)?;
    let published = values
        .publish(value, super::HOST_PAYLOAD_BYTES)
        .map_err(|_| super::invalid("component host recording failed; dispatch is not safe to retry"))?;
    if published.value_ref != value.value_ref || published.byte_count != value.bytes.len() as u64 {
        return Err(super::invalid("component host recording port returned a different publication"));
    }
    crate::preserves_rail::validate_content_ref(&published.publication_ref).map_err(super::convert)
}

#[cfg(test)]
mod tests;
