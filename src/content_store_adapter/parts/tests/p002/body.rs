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

    let aborted = execute_simulated_put(SimulatedStreamInput {
        fault: Some(Fault::MultipartAbortAt(1)),
        ..input
    })
    .expect("multipart abort outcome");
    assert_eq!(aborted.state.artifact.terminal, ContentTerminal::Failed);
    assert!(aborted.verified_chunks.is_empty());

    std::fs::remove_dir_all(workspace).expect("remove simulation put fixture");
}
