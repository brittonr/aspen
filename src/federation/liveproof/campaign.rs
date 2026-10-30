pub fn loopback(input: &super::RunInput, fault: &str) -> crate::error::Result<preserves::IOValue> {
    let root = &input.run_directory;
    let exchange_root = crate::node_state::DirectoryRoot::open(root)?;
    let exchange = exchange_root.receipts()?;
    super::require_fresh(&exchange)?;
    super::configure(&exchange, input)?;
    select(&exchange, fault)?;
    let provider_root = crate::node_state::DirectoryRoot::open(&root.join("provider"))?;
    let receiver_root = crate::node_state::DirectoryRoot::open(&root.join("receiver"))?;
    let provider_exchange = exchange_root.receipts()?;
    let child = std::thread::spawn(move || {
        let outcome = super::provider::run(&provider_root, &provider_exchange);
        (provider_root, outcome)
    });
    let received = super::receiver::run(&receiver_root, &exchange);
    let (provider_root, _provided) = child.join().map_err(|_| super::invalid("malicious provider thread panicked"))?;
    if received.is_ok() {
        return Err(super::invalid("malicious response unexpectedly completed receiver import"));
    }
    finish(&receiver_root, &provider_root, fault, "live-loopback", &exchange)
}
pub fn distinct_process(
    input: &super::RunInput,
    binary: &std::path::Path,
    fault: &str,
) -> crate::error::Result<preserves::IOValue> {
    let root = &input.run_directory;
    let exchange_root = crate::node_state::DirectoryRoot::open(root)?;
    let exchange = exchange_root.receipts()?;
    super::require_fresh(&exchange)?;
    super::configure(&exchange, input)?;
    select(&exchange, fault)?;
    let mut provider = super::spawn(binary, root, "provider")?;
    let mut receiver = super::spawn(binary, root, "receiver")?;
    let mut deadline = crate::fabric_time::SupervisionDeadline::after(super::setup::transport::TIMEOUT * 3)?;
    let _provided = provider.wait(&mut deadline)?;
    let received = receiver.wait(&mut deadline)?;
    if received.success() {
        return Err(super::invalid("malicious response unexpectedly completed receiver process"));
    }
    let provider_handoff = super::setup::exchange::handoff(&exchange, "provider")?;
    let receiver_handoff = super::setup::exchange::handoff(&exchange, "receiver")?;
    if provider_handoff.process_id != u64::from(provider.child.id())
        || receiver_handoff.process_id != u64::from(receiver.child.id())
        || provider.child.id() == receiver.child.id()
    {
        return Err(super::invalid("malicious campaign roles do not match distinct OS process identities"));
    }
    finish(
        &crate::node_state::DirectoryRoot::open_existing(&root.join("receiver"))?,
        &crate::node_state::DirectoryRoot::open_existing(&root.join("provider"))?,
        fault,
        "distinct-os-process",
        &exchange,
    )
}
fn select(exchange: &crate::node_state::Scope, fault: &str) -> crate::error::Result<()> {
    if ![
        "corrupt-signature",
        "wrong-key",
        "wrong-domain",
        "corrupt-content",
        "unsolicited-push",
    ]
    .contains(&fault)
    {
        return Err(super::invalid("unsupported actual-wire campaign fault"));
    }
    super::setup::dagports::persist(exchange, "fixture-fault.preserves", &crate::preserves_rail::string(fault))
        .map(|_| ())
}
fn finish(
    receiver: &crate::node_state::DirectoryRoot,
    provider: &crate::node_state::DirectoryRoot,
    fault: &str,
    mode: &str,
    exchange: &crate::node_state::Scope,
) -> crate::error::Result<preserves::IOValue> {
    let head = crate::preserves_rail::canonical_hash(&super::setup::profiles::artifact())?;
    let registry = receiver.ledger_store()?;
    let path = crate::local_store::Locator::parse(&format!(
        "content/blake3_{}.bin",
        crate::preserves_rail::content_ref_hex(&head)?
    ))?;
    if registry.root().try_exists(&path)? || crate::ledger::read_artifact_with_root(&registry, &head).is_ok() {
        return Err(super::invalid("negative fixture mutated the receiver artifact registry"));
    }
    let provider_process = super::setup::exchange::handoff(exchange, "provider")?.process_id;
    let receiver_process = super::setup::exchange::handoff(exchange, "receiver")?.process_id;
    let observed = crate::mesh::envelope::boundary::read(&receiver.receipts()?)?;
    let denial = observed.value();
    let malicious =
        crate::preserves_rail::parse_canonical_bytes(&provider.receipts()?.read(
            &crate::node_state::RelativePath::parse("malicious-wire.preserves")?,
            super::setup::transport::LIMIT,
        )?)?;
    let wire_ref = packet_ref(&malicious, fault)?;
    let expected = match fault {
        "corrupt-signature" | "wrong-key" => crate::mesh::envelope::boundary::FailureClass::SignatureRejected,
        "wrong-domain" => crate::mesh::envelope::boundary::FailureClass::RecordRejected,
        "corrupt-content" => crate::mesh::envelope::boundary::FailureClass::ContentIdentityRejected,
        "unsolicited-push" => crate::mesh::envelope::boundary::FailureClass::CorrelationRejected,
        _ => return Err(super::invalid("unknown boundary fault")),
    };
    let session = super::setup::exchange::wait(exchange, "receiver-session.preserves")?;
    if observed.class != expected
        || observed.wire_ref != wire_ref
        || observed.generation != 1
        || observed.process_id != receiver_process
        || session.as_string().as_deref() != Some(observed.session_ref.as_str())
    {
        return Err(super::invalid(
            "actual boundary rejection does not match transmitted wire, receiver session and process",
        ));
    }
    let value = crate::preserves_rail::record("federation-live-boundary-denial-v1", vec![
        crate::preserves_rail::string(mode),
        crate::preserves_rail::string(fault),
        crate::preserves_rail::string(head),
        crate::preserves_rail::string(wire_ref),
        malicious,
        denial,
        crate::preserves_rail::record("federation-live-role-processes-v1", vec![
            crate::preserves_rail::u64_value(provider_process),
            crate::preserves_rail::u64_value(receiver_process),
        ]),
        crate::preserves_rail::string("no imported artifact after actual receiver boundary rejection"),
    ]);
    super::setup::dagports::persist(&receiver.receipts()?, "boundary-denial.preserves", &value)?;
    Ok(value)
}
fn packet_ref(value: &preserves::IOValue, fault: &str) -> crate::error::Result<String> {
    let packet = value
        .collect_simple_record("federation-live-malicious-wire-v1", Some(3))
        .ok_or_else(|| super::invalid("missing actual transmitted packet receipt"))?;
    let wire_ref = crate::preserves_rail::canonical_hash(&crate::preserves_rail::value_to_iovalue(&packet[2]))?;
    if packet[0].as_string().as_deref() != Some(fault) || packet[1].as_string().as_deref() != Some(wire_ref.as_str()) {
        return Err(super::invalid("negative packet receipt differs from actual transmitted canonical bytes"));
    }
    Ok(wire_ref)
}

/// Only the executable fixture owns malicious-wire transformations.
/// Every source object and every signature still comes from the real production adapters.
pub fn transform(
    root: &crate::node_state::DirectoryRoot,
    exchange: &crate::node_state::Scope,
    adapter: &crate::fabric_crypto_identity::IrohEd25519<'_>,
    prepared: &mut crate::mesh::transfer::provider::PreparedResponse,
) -> crate::error::Result<()> {
    let path = crate::node_state::RelativePath::parse("fixture-fault.preserves")?;
    if !exchange.try_exists(&path)? {
        return Ok(());
    }
    let value = crate::preserves_rail::parse_canonical_bytes(&exchange.read(&path, 256)?)?;
    let fault = value.as_string().ok_or_else(|| super::invalid("invalid fixture fault selection"))?;
    let policy = super::setup::transport::reference("federation-live-signature-policy");
    let context = super::setup::transport::reference("federation-live-signature-context");
    let key =
        adapter.resolve_or_generate(crate::fabric_crypto_identity::KeyPurpose::FederationOrigin, &policy, false)?;
    let mut payload = prepared.signed.payload.clone();
    match fault.as_ref() {
        "corrupt-signature" => prepared.signed.signature.signature[0] ^= 1,
        "wrong-key" => prepared.signed = wrong_key(root, &payload, &context, &policy)?,
        "wrong-domain" => {
            prepared.signed = crate::mesh::envelope::signatures::sign_record(
                adapter,
                &crate::mesh::envelope::signatures::SigningInput {
                    payload: &payload,
                    ..super::setup::exchange::signing(
                        adapter,
                        &key.handle.handle,
                        molten_core::federation::RecordPurpose::Announcement,
                        &context,
                        &policy,
                    )
                },
            )?
        }
        "corrupt-content" | "unsolicited-push" => {
            payload = corrupt(&payload, fault.as_ref())?;
            prepared.signed = crate::mesh::envelope::signatures::sign_record(
                adapter,
                &crate::mesh::envelope::signatures::SigningInput {
                    payload: &payload,
                    ..super::setup::exchange::signing(
                        adapter,
                        &key.handle.handle,
                        molten_core::federation::RecordPurpose::Response,
                        &context,
                        &policy,
                    )
                },
            )?;
        }
        _ => return Err(super::invalid("unknown fixture fault selection")),
    }
    let wire = crate::mesh::envelope::wire::encode(&prepared.signed, prepared.maximum_bytes)?;
    let evidence = crate::preserves_rail::record("federation-live-malicious-wire-v1", vec![
        crate::preserves_rail::string(fault.as_ref()),
        crate::preserves_rail::string(crate::preserves_rail::content_ref_from_bytes(&wire)),
        crate::preserves_rail::parse_canonical_bytes(&wire)?,
    ]);
    super::setup::dagports::persist(&root.receipts()?, "malicious-wire.preserves", &evidence).map(|_| ())
}
fn wrong_key(
    root: &crate::node_state::DirectoryRoot,
    payload: &preserves::IOValue,
    context: &str,
    policy: &str,
) -> crate::error::Result<crate::mesh::envelope::signatures::SignedRecord> {
    let secrets = root.secrets()?.open_subdir(&crate::node_state::RelativePath::parse("wrong-origin")?)?;
    let wrong = super::setup::profiles::crypto(&secrets)?;
    let key = wrong.resolve_or_generate(crate::fabric_crypto_identity::KeyPurpose::FederationOrigin, policy, true)?;
    crate::mesh::envelope::signatures::sign_record(&wrong, &crate::mesh::envelope::signatures::SigningInput {
        payload,
        ..super::setup::exchange::signing(
            &wrong,
            &key.handle.handle,
            molten_core::federation::RecordPurpose::Response,
            context,
            policy,
        )
    })
}
fn corrupt(payload: &preserves::IOValue, fault: &str) -> crate::error::Result<preserves::IOValue> {
    let fields = payload
        .collect_simple_record("federation-object-v1", Some(4))
        .ok_or_else(|| super::invalid("prepared response is malformed"))?;
    let mut fields = fields.fields_iter().map(crate::preserves_rail::value_to_iovalue).collect::<Vec<_>>();
    if fault == "unsolicited-push" {
        fields[0] = crate::preserves_rail::string(super::setup::transport::reference("unrequested-push"));
    } else {
        let bytes = fields[3].collect_sequence().ok_or_else(|| super::invalid("prepared bytes are malformed"))?;
        let mut bytes = bytes.iter().map(crate::preserves_rail::value_to_iovalue).collect::<Vec<_>>();
        let first = bytes.first_mut().ok_or_else(|| super::invalid("prepared content is empty"))?;
        let byte = first.as_u64().and_then(|value| value.ok()).ok_or_else(|| super::invalid("invalid content byte"))?;
        *first = crate::preserves_rail::u64_value(byte ^ 1);
        fields[3] = crate::preserves_rail::sequence(bytes);
    }
    Ok(crate::preserves_rail::record("federation-object-v1", fields))
}
