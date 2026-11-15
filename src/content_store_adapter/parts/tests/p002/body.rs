// r[verify molten.content_store_adapter.partial_state]
// r[verify molten.content_store_adapter.live_sim_conformance]
#[test]
fn simulated_put_classifies_throttle_torn_put_and_multipart_abort() {
    let (workspace, _root, manifest) = fixture("content-adapter-simulation-put");
    let simulation_profile = profile(ContentAdapterClass::DeterministicSimulation);
    let command = command(&simulation_profile, &manifest, ContentOperation::Put, None);
    let chunks = fixture_chunks(&manifest);
    let input = SimulatedStreamInput {
        profile: &simulation_profile,
        manifest: &manifest,
        command: &command,
        generation: GENERATION_ONE,
        retained: None,
        chunks: &chunks,
        fault: None,
    };

    let durable = execute_simulated_put(input).expect("durable put");
    assert_eq!(durable.state.artifact.terminal, ContentTerminal::Durable);

    let throttled = execute_simulated_put(SimulatedStreamInput {
        fault: Some(Fault::ProviderThrottleAt(1)),
        ..input
    })
    .expect("throttle outcome");
    assert_eq!(throttled.state.artifact.terminal, ContentTerminal::Retryable);
    assert!(throttled.verified_chunks.is_empty());

    let torn = execute_simulated_put(SimulatedStreamInput {
        fault: Some(Fault::TornPutAt(1)),
        ..input
    })
    .expect("torn put outcome");
    assert_eq!(torn.state.artifact.terminal, ContentTerminal::Uncertain);
    assert!(torn.verified_chunks.is_empty());
    assert!(execute_simulated_put(SimulatedStreamInput { retained: Some(&torn.state.artifact), ..input }).is_err(),
        "uncertain puts must not silently resume");

    let aborted = execute_simulated_put(SimulatedStreamInput {
        fault: Some(Fault::MultipartAbortAt(1)),
        ..input
    })
    .expect("multipart abort outcome");
    assert_eq!(aborted.state.artifact.terminal, ContentTerminal::Failed);
    assert!(aborted.verified_chunks.is_empty());
    for (fault, expected) in [
        (Fault::DisconnectAt(0), ContentTerminal::Retryable),
        (Fault::CorruptAt(0), ContentTerminal::Failed),
        (Fault::TruncateAt(0), ContentTerminal::Failed),
        (Fault::LatencyTicks(SIMULATION_TIMEOUT_LATENCY), ContentTerminal::Uncertain),
        (Fault::CancelAt(0), ContentTerminal::Cancelled),
    ] {
        let result = execute_simulated_put(SimulatedStreamInput { fault: Some(fault), ..input }).expect("put fault");
        assert_eq!(result.state.artifact.terminal, expected);
        assert!(result.verified_chunks.is_empty());
    }
    let get_command = ContentCommand { operation: ContentOperation::Get, ..command.clone() };
    let get_input = SimulatedStreamInput { command: &get_command, ..input };
    let throttled_get = execute_simulated_stream(SimulatedStreamInput {
        fault: Some(Fault::ProviderThrottleAt(0)), ..get_input
    }).expect("get throttle");
    assert_eq!(throttled_get.state.artifact.terminal, ContentTerminal::Retryable);
    assert!(throttled_get.verified_chunks.is_empty());
    assert!(execute_simulated_stream(SimulatedStreamInput { fault: Some(Fault::TornPutAt(0)), ..get_input }).is_err(),
        "a put-only fault must not silently become a successful get");
    retain_put_fault_receipt(&simulation_profile, &manifest, &command, [
        ("provider-throttle-at-1", &throttled), ("torn-put-at-1", &torn), ("multipart-abort-at-1", &aborted),
    ]);
    std::fs::remove_dir_all(workspace).expect("remove simulation put fixture");
}

fn retain_put_fault_receipt(
    simulation_profile: &ContentAdapterProfile,
    manifest: &ContentManifestDescriptor,
    command: &ContentCommand,
    scenarios: [(&str, &ModelExecution); 3],
) {
    let command_artifact = bind_command(simulation_profile, manifest, command, 0, 0).expect("canonical command");
    let mut rows = Vec::with_capacity(scenarios.len());
    for (fault, execution) in scenarios {
        let fixture_command = crate::preserves_rail::record("simulated-put-fixture-command-v1", vec![
            crate::preserves_rail::string(fault), command_artifact.value.clone(), crate::preserves_rail::u64_value(1),
        ]);
        rows.push(crate::preserves_rail::record("fault-outcome", vec![
            crate::preserves_rail::string(fault),
            crate::preserves_rail::string(crate::preserves_rail::canonical_hash(&fixture_command).expect("fixture digest")),
            crate::preserves_rail::string(execution.state.artifact.terminal.as_str()),
            crate::preserves_rail::string(&execution.state.artifact_ref),
            crate::preserves_rail::sequence(execution.events.iter().map(|event| crate::preserves_rail::string(&event.artifact_ref)).collect()),
        ]));
    }
    let receipt = crate::preserves_rail::record("content-store-put-fault-receipt-v1", vec![
        crate::preserves_rail::string(&simulation_profile.profile_ref),
        crate::preserves_rail::string(&manifest.manifest_ref), crate::preserves_rail::sequence(rows),
        crate::preserves_rail::string("simulation-only-no-live-s3-or-performance-claim"),
    ]);
    if let Ok(root) = std::env::var("MOLTEN_CONTENT_RECEIPT_DIR") {
        let namespace = crate::node_state::Scope::open_existing(crate::node_state::NamespaceKind::Receipts, std::path::Path::new(&root))
            .expect("explicit receipt directory");
        namespace.write(&crate::node_state::RelativePath::parse("put-faults.preserves").expect("receipt locator"),
            &crate::preserves_rail::canonical_bytes(&receipt).expect("canonical receipt")).expect("persist actual receipt");
        println!("put-faults receipt {}", crate::preserves_rail::canonical_hash(&receipt).expect("receipt digest"));
    }
}
