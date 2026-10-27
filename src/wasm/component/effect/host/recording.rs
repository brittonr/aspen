//! Exact offline playback. This API deliberately accepts no live effect port.

pub struct Replay {
    session: super::Session,
}

pub struct ReplayInput<'a> {
    pub request_bytes: &'a [u8],
    pub expected_record_ref: &'a str,
    pub record: &'a crate::system_extension::NativeCallbackValue,
    pub response_bytes: &'a [u8],
}

impl Replay {
    pub fn new(scope: super::Scope, generation: u64, incarnation: u64) -> super::Result<Self> {
        Ok(Self {
            session: super::Session::new(scope, generation, incarnation)?,
        })
    }

    // r[impl molten.wasm_component_adoption.hostcalls]
    pub fn consume(&mut self, input: ReplayInput<'_>) -> super::Result<super::super::HostResponse> {
        if self.session.is_poisoned {
            return Err(super::super::invalid("component host replay stopped at its first divergence"));
        }
        self.session.is_poisoned = true;
        let response = self.validate(input)?;
        self.session.next_sequence = self
            .session
            .next_sequence
            .checked_add(1)
            .ok_or_else(|| super::super::invalid("component host replay sequence overflow"))?;
        self.session.is_poisoned = false;
        Ok(response)
    }

    fn validate(&self, input: ReplayInput<'_>) -> super::Result<super::super::HostResponse> {
        let request = super::super::decode_host_request(input.request_bytes)?;
        if request.sequence() != self.session.next_sequence || request.sequence() >= 1_024 {
            return Err(super::super::invalid("component host replay sequence differs from the recorded boundary"));
        }
        super::super::bounded(&input.record.bytes)?;
        if input.record.value_ref != input.expected_record_ref {
            return Err(super::super::invalid("component host replay completion is not the selected record"));
        }
        crate::system_extension::admit_callback_value(input.record, super::super::HOST_PAYLOAD_BYTES)
            .map_err(|_| super::super::invalid("component host replay record identity is stale"))?;
        let record =
            crate::preserves_rail::parse_canonical_bytes(&input.record.bytes).map_err(super::super::convert)?;
        let fields = crate::preserves_rail::simple_record_fields(&record, "component-host-completion-v1", 8)
            .map_err(super::super::convert)?;
        let intent = crate::preserves_rail::required_content_ref(&fields[6], "component host replay intent")
            .map_err(super::super::convert)?;
        let response = super::super::decode_host_response(input.response_bytes, &request)?;
        if !response.response().evidence_refs.iter().any(|reference| reference == intent.as_str()) {
            return Err(super::super::invalid("component host replay response names another intent"));
        }
        let response_ref = crate::preserves_rail::content_ref_from_bytes(input.response_bytes);
        let expected = completion(&self.session, &request, &response_ref, intent.as_str())?;
        if crate::preserves_rail::canonical_hash(&expected).map_err(super::super::convert)? != input.expected_record_ref
        {
            return Err(super::super::invalid("component host replay scope, lifecycle, request or response diverged"));
        }
        Ok(response)
    }
}

pub(super) fn completion(
    session: &super::Session,
    request: &super::super::HostRequest,
    response_ref: &str,
    intent_ref: &str,
) -> super::Result<preserves::IOValue> {
    Ok(crate::preserves_rail::record("component-host-completion-v1", vec![
        super::context::scope_value(&session.scope),
        crate::preserves_rail::u64_value(session.generation),
        crate::preserves_rail::u64_value(session.incarnation),
        crate::preserves_rail::u64_value(request.sequence()),
        crate::preserves_rail::string(request.request_ref()),
        crate::preserves_rail::string(response_ref),
        crate::preserves_rail::string(intent_ref),
        crate::preserves_rail::string(super::context::recording_ref(
            &session.scope,
            session.generation,
            session.incarnation,
        )?),
    ]))
}

pub(super) fn entry(
    scope: &super::Scope,
    request: &super::super::HostRequest,
    response_ref: String,
) -> crate::deterministic_replay::EffectLogEntry {
    crate::deterministic_replay::EffectLogEntry {
        sequence: request.sequence(),
        effect_kind: "blob-get".to_string(),
        run_identity_ref: scope.run_ref.clone(),
        handler_profile_ref: scope.handler_profile_ref.clone(),
        turn_ref: scope.turn_ref.clone(),
        boundary_ref: scope.boundary_ref.clone(),
        request_ref: request.request_ref().to_string(),
        response_ref,
    }
}
