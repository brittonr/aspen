pub const MAX_OPERATION_BYTES: usize = 1_048_576;

pub enum Invocation {
    Install(String),
    Status,
    Supervise {
        group: String,
        exit: crate::system_extension::RecoveryExit,
    },
    Submit {
        member: String,
        work: crate::system_extension::WorkPresentation,
        payload: crate::system_extension::NativeCallbackValue,
    },
    Readmit {
        member: String,
        work_ref: String,
    },
    Migrate {
        member: String,
        input: crate::node_service_hosting::MigratedWorkInput,
    },
    Deliver(Box<crate::node_service_hosting::DeliveryInput>),
    Upgrade {
        member: String,
        replacement_ref: String,
    },
    Reconcile(String),
}

pub fn decode(bytes: &[u8], command: &str, logical_tick: u64) -> crate::error::Result<Invocation> {
    if bytes.len() > MAX_OPERATION_BYTES {
        return Err(super::invalid("service host operation exceeds byte bound"));
    }
    let value = crate::preserves_rail::strict_canonical_decode(bytes)?.value;
    let tag = format!("node-service-host-{command}-v1");
    let count = match command {
        "status" => 0,
        "install" | "reconcile" => 1,
        "readmit" | "upgrade" => 2,
        "submit" | "deliver" => 3,
        "supervise" => 4,
        _ => return Err(super::invalid("unknown service host command")),
    };
    let fields: Vec<Field> =
        crate::preserves_rail::simple_record_fields(&value, &tag, count)?.fields_iter().cloned().collect();
    match command {
        "status" => Ok(Invocation::Status),
        "install" => Ok(Invocation::Install(text(&fields[0])?)),
        "reconcile" => Ok(Invocation::Reconcile(text(&fields[0])?)),
        "upgrade" => Ok(Invocation::Upgrade {
            member: text(&fields[0])?,
            replacement_ref: reference(&fields[1])?,
        }),
        "submit" => Ok(Invocation::Submit {
            member: text(&fields[0])?,
            work: work(&fields[1])?,
            payload: payload(&fields[2])?,
        }),
        "readmit" => readmit(&fields),
        "supervise" => supervise(&fields),
        "deliver" => delivery(&fields, logical_tick),
        _ => Err(super::invalid("unknown service host operation")),
    }
}

type Field = preserves::Value<preserves::IOValue>;

fn readmit(fields: &[Field]) -> crate::error::Result<Invocation> {
    let member = text(&fields[0])?;
    let value = crate::preserves_rail::value_to_iovalue(&fields[1]);
    if fields[1].as_string().is_some() {
        return Ok(Invocation::Readmit {
            member,
            work_ref: reference(&fields[1])?,
        });
    }
    let input = crate::preserves_rail::simple_record_fields(&value, "node-service-host-migrated-work-v1", 3)?;
    Ok(Invocation::Migrate {
        member,
        input: crate::node_service_hosting::MigratedWorkInput {
            work_ref: reference(&input[0])?,
            migration_ref: reference(&input[1])?,
            payload: payload(&input[2])?,
        },
    })
}

fn supervise(fields: &[Field]) -> crate::error::Result<Invocation> {
    let failure = match text(&fields[3])?.as_str() {
        "none" => None,
        "retryable" => Some(crate::system_extension::FailureClass::Retryable),
        "fatal" => Some(crate::system_extension::FailureClass::Fatal),
        "policy-violation" => Some(crate::system_extension::FailureClass::PolicyViolation),
        "resource-violation" => Some(crate::system_extension::FailureClass::ResourceViolation),
        "generation-violation" => Some(crate::system_extension::FailureClass::GenerationViolation),
        _ => return Err(super::invalid("invalid recovery failure class")),
    };
    let was_successful = fields[2].as_boolean().ok_or_else(|| super::invalid("exit observation requires boolean"))?;
    Ok(Invocation::Supervise {
        group: text(&fields[0])?,
        exit: crate::system_extension::RecoveryExit {
            member_id: text(&fields[1])?,
            was_successful,
            failure,
        },
    })
}

fn delivery(fields: &[Field], logical_tick: u64) -> crate::error::Result<Invocation> {
    let value = crate::preserves_rail::value_to_iovalue(&fields[2]);
    let input = crate::preserves_rail::simple_record_fields(&value, "node-service-host-delivery-v1", 2)?;
    let claims = crate::preserves_rail::required_sequence_field(&input[0], "delivery claim")?;
    if claims.len() > 1 {
        return Err(super::invalid("delivery requires at most one claim"));
    }
    Ok(Invocation::Deliver(Box::new(crate::node_service_hosting::DeliveryInput {
        member: text(&fields[0])?,
        presentation: work(&fields[1])?,
        claim_ref: claims.first().map(reference).transpose()?,
        ingress: ingress(&input[1])?,
        logical_tick,
    })))
}

fn work(value: &Field) -> crate::error::Result<crate::system_extension::WorkPresentation> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = crate::preserves_rail::simple_record_fields(&value, "node-service-host-work-v1", 4)?;
    let class = match text(&fields[1])?.as_str() {
        "durable" => crate::system_extension::WorkClass::Durable,
        "runtime-local" => crate::system_extension::WorkClass::RuntimeLocal,
        _ => return Err(super::invalid("invalid work class")),
    };
    Ok(crate::system_extension::WorkPresentation {
        work_ref: reference(&fields[0])?,
        class,
        generation: number(&fields[2])?,
        incarnation: number(&fields[3])?,
    })
}

fn payload(value: &Field) -> crate::error::Result<crate::system_extension::NativeCallbackValue> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = crate::preserves_rail::simple_record_fields(&value, "node-service-host-value-v1", 2)?;
    let values = crate::preserves_rail::required_sequence_field(&fields[1], "materialized payload")?;
    if values.len() > crate::node_service_hosting::MAX_HOST_PAYLOAD_BYTES {
        return Err(super::invalid("materialized payload exceeds byte bound"));
    }
    let bytes = values
        .iter()
        .map(|field| u8::try_from(number(field)?).map_err(|_| super::invalid("payload value is outside byte range")))
        .collect::<crate::error::Result<Vec<_>>>()?;
    let payload = crate::system_extension::NativeCallbackValue {
        value_ref: reference(&fields[0])?,
        bytes,
    };
    crate::system_extension::admit_callback_value(&payload, crate::node_service_hosting::MAX_HOST_PAYLOAD_BYTES as u64)
        .map_err(|error| super::invalid(format!("{error:?}")))?;
    Ok(payload)
}

fn ingress(value: &Field) -> crate::error::Result<crate::system_extension::NativeIngressEnvelope> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = crate::preserves_rail::simple_record_fields(&value, "node-service-host-ingress-v1", 13)?;
    Ok(crate::system_extension::NativeIngressEnvelope {
        schema: crate::system_extension::NATIVE_INGRESS_SCHEMA.to_string(),
        request_ref: reference(&fields[0])?,
        endpoint_ref: reference(&fields[1])?,
        peer_ref: reference(&fields[2])?,
        service_id: text(&fields[3])?,
        manifest_ref: reference(&fields[4])?,
        generation: number(&fields[5])?,
        authority_ref: reference(&fields[6])?,
        policy_ref: reference(&fields[7])?,
        resource_ref: reference(&fields[8])?,
        transport_profile_ref: reference(&fields[9])?,
        alpn: crate::system_extension::NATIVE_ALPN.to_string(),
        framing: crate::system_extension::NATIVE_FRAMING.to_string(),
        payload: payload(&fields[10])?,
        accounted_bytes: number(&fields[11])?,
    })
    .and_then(|ingress| {
        if text(&fields[12])? != "materialized-native-v3" {
            return Err(super::invalid("unsupported ingress protocol"));
        }
        Ok(ingress)
    })
}

fn text(value: &Field) -> crate::error::Result<String> {
    let text = crate::preserves_rail::required_string_field(value, "service host operation field")?;
    if text.len() > 4096 {
        return Err(super::invalid("operation text exceeds bound"));
    }
    Ok(text)
}

fn reference(value: &Field) -> crate::error::Result<String> {
    crate::preserves_rail::required_content_ref_string(value, "service host operation reference")
}

fn number(value: &Field) -> crate::error::Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| super::invalid("operation field requires u64"))?
        .map_err(|_| super::invalid("operation integer outside u64 range"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn command_substitution_and_trailing_bytes_are_rejected() {
        let value = crate::preserves_rail::record("node-service-host-status-v1", Vec::new());
        let mut bytes = crate::preserves_rail::canonical_bytes(&value).expect("canonical status");
        assert!(matches!(super::decode(&bytes, "status", 7), Ok(super::Invocation::Status)));
        assert!(super::decode(&bytes, "install", 7).is_err());
        bytes.push(0);
        assert!(super::decode(&bytes, "status", 7).is_err());
    }

    #[test]
    fn payload_reference_cannot_substitute_different_materialized_bytes() {
        let value = crate::preserves_rail::record("node-service-host-value-v1", vec![
            crate::preserves_rail::string(crate::preserves_rail::content_ref_from_bytes(b"admitted")),
            crate::preserves_rail::sequence(vec![crate::preserves_rail::u64_value(0)]),
        ]);
        let envelope = crate::preserves_rail::record("test-envelope", vec![value]);
        let fields =
            crate::preserves_rail::simple_record_fields(&envelope, "test-envelope", 1).expect("payload envelope");
        assert!(super::payload(&fields[0]).is_err());
    }
}
