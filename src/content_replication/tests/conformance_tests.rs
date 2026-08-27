use super::*;

#[test]
fn deterministic_durability_commits_or_crashes_before_progress() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let manifest = manifest();
    let mut facts = FactPorts::admitted(&events);
    let instance = active(manifest.clone(), &mut facts);
    let mut effects = EffectPorts::admitted(&manifest, &events);
    let mut durable = SimulatedDurableReplicationAdapter::open(&manifest, None).expect("durable adapter");
    let complete = reconcile(instance.clone(), durable_ports(&mut facts, &mut effects, &mut durable))
        .expect("durable reconciliation");
    assert_eq!(complete.receipt.decision, ReceiptDecision::Complete);
    assert_eq!(durable.history().len(), 1);

    let mut crashed = SimulatedDurableReplicationAdapter::open(
        &manifest,
        Some(crate::fabric_durability::SimulatedDurabilityFault::CrashBeforeMutation),
    )
    .expect("crashing adapter");
    let mut crash_effects = EffectPorts::admitted(&manifest, &events);
    let result = reconcile(instance, durable_ports(&mut facts, &mut crash_effects, &mut crashed));
    assert!(result.is_err());
    assert!(crashed.history().is_empty());
    assert_eq!(crash_effects.receipts.count, 0);
}

#[cfg(unix)]
#[test]
fn deterministic_adapters_classify_transport_content_and_disk_faults() {
    let faults = [
        TransferFault::CancelAt { call: 0 },
        TransferFault::PartitionAt { call: 0 },
        TransferFault::TimeoutAt { call: 0 },
        TransferFault::UnavailableAt { call: 0 },
    ];
    for fault in faults {
        let events = Rc::new(RefCell::new(Vec::new()));
        let manifest = manifest();
        let mut facts = FactPorts::admitted(&events);
        let instance = active(manifest.clone(), &mut facts);
        let mut effects = EffectPorts::admitted(&manifest, &events);
        let mut content = SimulatedContent::new(&manifest, &events, None).expect("simulated content");
        let mut transport =
            FabricTransferAdapter::open(&manifest, TransferProfile::DeterministicSimulation, Some(fault))
                .expect("faulted transport");
        let outcome = reconcile(instance, conformance_ports(&mut facts, &mut effects, &mut content, &mut transport))
            .expect("classified transport fault");
        assert_eq!(outcome.receipt.decision, ReceiptDecision::Partial);
        assert_eq!(effects.durable.stored.len(), 1);
        assert_eq!(transport.call_count(), 1);
    }

    let events = Rc::new(RefCell::new(Vec::new()));
    let manifest = manifest();
    let mut facts = FactPorts::admitted(&events);
    let instance = active(manifest.clone(), &mut facts);
    let mut effects = EffectPorts::admitted(&manifest, &events);
    let mut content =
        SimulatedContent::new(&manifest, &events, Some(crate::content_store_adapter::SimulationFault::CorruptAt(0)))
            .expect("corrupt content adapter");
    let mut transport = FabricTransferAdapter::open(&manifest, TransferProfile::DeterministicSimulation, None)
        .expect("simulated transport");
    let corrupt = reconcile(instance, conformance_ports(&mut facts, &mut effects, &mut content, &mut transport));
    assert!(corrupt.is_err());
    assert!(effects.durable.stored.is_empty());
}

#[cfg(unix)]
#[test]
fn same_core_simulated_content_and_live_iroh_local_content_agree() {
    let seed_events = Rc::new(RefCell::new(Vec::new()));
    let mut manifest = manifest();
    let mut local_content = LocalContent::new(&mut manifest, &seed_events).expect("local content adapter");
    let mut simulated_content =
        SimulatedContent::new(&manifest, &seed_events, None).expect("simulated content adapter");

    let simulated_events = Rc::new(RefCell::new(Vec::new()));
    let mut simulated_facts = FactPorts::admitted(&simulated_events);
    let simulated_instance = active(manifest.clone(), &mut simulated_facts);
    let mut simulated_effects = EffectPorts::admitted(&manifest, &simulated_events);
    let mut simulated_transport =
        FabricTransferAdapter::open(&manifest, TransferProfile::DeterministicSimulation, None)
            .expect("simulated transport");
    let simulated = reconcile(
        simulated_instance,
        conformance_ports(
            &mut simulated_facts,
            &mut simulated_effects,
            &mut simulated_content,
            &mut simulated_transport,
        ),
    )
    .expect("simulated reconciliation");

    let live_events = Rc::new(RefCell::new(Vec::new()));
    let mut live_facts = FactPorts::admitted(&live_events);
    let live_instance = active(manifest.clone(), &mut live_facts);
    let mut live_effects = EffectPorts::admitted(&manifest, &live_events);
    let mut live_transport =
        FabricTransferAdapter::open(&manifest, TransferProfile::IrohLiveLoopback, None).expect("live transport");
    let live = reconcile(
        live_instance,
        conformance_ports(&mut live_facts, &mut live_effects, &mut local_content, &mut live_transport),
    )
    .expect("live reconciliation");

    assert_eq!(simulated.plan, live.plan);
    assert_eq!(simulated.status, live.status);
    assert_eq!(simulated.receipt.decision, live.receipt.decision);
    assert_eq!(simulated.receipt.decision, ReceiptDecision::Complete);
    assert_eq!(simulated_transport.call_count(), live_transport.call_count());
}
