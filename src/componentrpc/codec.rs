//! Strict canonical transcript admission for offline replay; no live authority is reconstructed.

fn text(fields: &preserves::Record<preserves::Value<preserves::IOValue>>, index: usize) -> super::Result<String> {
    crate::codec::required_string_field(&fields[index], "component RPC field").map_err(super::failure)
}
fn number(fields: &preserves::Record<preserves::Value<preserves::IOValue>>, index: usize) -> super::Result<u64> {
    text(fields, index)?.parse().map_err(super::failure)
}
fn scope(value: &preserves::IOValue) -> super::Result<super::bridge::Scope> {
    let fields = crate::codec::simple_record_fields(value, "component-rpc-scope-v1", 15).map_err(super::failure)?;
    if text(&fields, 0)? != super::PROFILE || text(&fields, 1)? != super::REVISION {
        return Err("transcript profile or runtime revision differs".into());
    }
    let scope = super::bridge::Scope {
        caller: text(&fields, 2)?,
        callee: text(&fields, 3)?,
        wit: text(&fields, 4)?,
        bundle: text(&fields, 5)?,
        policy: text(&fields, 6)?,
        resources: text(&fields, 7)?,
        peer: text(&fields, 8)?,
        caller_peer: text(&fields, 9)?,
        session: text(&fields, 10)?,
        delivery: text(&fields, 11)?,
        dedup: text(&fields, 12)?,
        generation: number(&fields, 13)?,
        deadline_tick: number(&fields, 14)?,
    };
    if scope.value() != *value {
        return Err("noncanonical scope projection".into());
    }
    Ok(scope)
}
fn evidence(value: &preserves::IOValue) -> super::Result<crate::fabric_transport::FrameEvidence> {
    let fields =
        crate::codec::simple_record_fields(value, "component-rpc-transport-evidence-v1", 13).map_err(super::failure)?;
    let role = match text(&fields, 0)?.as_str() {
        "client" => crate::fabric_transport::EndpointParticipantRole::Client,
        "listener" => crate::fabric_transport::EndpointParticipantRole::Listener,
        _ => return Err("unsupported transport role".into()),
    };
    if text(&fields, 1)? != "clean" || text(&fields, 9)? != "delivered" || text(&fields, 10)? != "not-applicable" {
        return Err("replay requires complete delivered evidence; uncertainty cannot be upgraded".into());
    }
    Ok(crate::fabric_transport::FrameEvidence {
        role,
        terminal_class: crate::fabric_transport::SessionTerminalClass::Clean,
        descriptor_ref: text(&fields, 2)?,
        session_ref: text(&fields, 3)?,
        request_ref: text(&fields, 4)?,
        payload_ref: text(&fields, 5)?,
        acknowledgement_ref: text(&fields, 6)?,
        remote_transport_identity_ref: text(&fields, 7)?,
        payload_bytes: number(&fields, 8)?,
        delivery: crate::fabric_transport::DeliveryOutcome::Delivered,
        retry: crate::fabric_transport::RetryDisposition::NotApplicable,
        automatic_retry_count: number(&fields, 11)?,
        cleanup_evidence_ref: text(&fields, 12)?,
    })
}

impl super::transcript::Call {
    pub fn from_value(value: &preserves::IOValue) -> super::Result<Self> {
        let fields =
            crate::codec::simple_record_fields(value, "component-rpc-transcript-v1", 7).map_err(super::failure)?;
        let scope = scope(&crate::codec::value_to_iovalue(&fields[0]))?;
        let invocation = super::bridge::invocation(&scope)?;
        if crate::codec::value_to_iovalue(&fields[4]) != super::bridge::request(&scope, &[])? {
            return Err("recorded request differs".into());
        }
        let event_value = crate::codec::value_to_iovalue(&fields[6]);
        let event_fields =
            crate::codec::simple_record_fields(&event_value, "ordered-events", 3).map_err(super::failure)?;
        let events = (0..3).map(|index| text(&event_fields, index)).collect::<super::Result<Vec<_>>>()?;
        let transcript = Self {
            scope,
            authority: text(&fields, 1)?,
            request: evidence(&crate::codec::value_to_iovalue(&fields[2]))?,
            response: evidence(&crate::codec::value_to_iovalue(&fields[3]))?,
            result_wire: super::bridge::decode_envelope(&crate::codec::value_to_iovalue(&fields[5]), &invocation)?,
            events,
            reference: super::hash(value)?,
        };
        if transcript.value()? != *value {
            return Err("noncanonical transcript projection".into());
        }
        transcript.validate()?;
        Ok(transcript)
    }
}
