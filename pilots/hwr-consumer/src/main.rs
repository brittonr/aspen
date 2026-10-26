//! Standalone Molten-owned boundary pilot; never called by the production host.

const EXPORT: &str = "actor-turn";
const WIT: &[u8] = include_bytes!("../wit/actor.wit");
const WIT_ID: &str = "e6b69908a1dcba063fd438d2cf44776ee5f7d7b7919f14aeefce766e72bfd82c";
const INPUT: [u8; 8] = [7, 0, 0, 0, 41, 0, 0, 0];
const EXPECTED: [u8; 8] = [7, 0, 0, 0, 42, 0, 0, 0];

// r[impl molten.hwr_consumer_pilot.boundary]
// r[impl molten.hwr_consumer_pilot.actor]
fn main() -> Result<(), String> {
    assert_eq!(blake3::hash(WIT).to_hex().as_str(), WIT_ID, "Molten WIT identity drift");
    let admitted = hardened_wasmtime_runtime::profile::document::admit(include_str!("../profile.json"))
        .map_err(|error| format!("Molten-owned pilot policy: {error:?}"))?;
    assert!(WIT.len() as u64 <= admitted.limits.max_wit_bytes);
    let runtime = hardened_wasmtime_shell::BoundedRuntime::build(
        &admitted.policy,
        admitted.limits,
        hardened_wasmtime_runtime::Strategy::Cranelift,
    )
    .map_err(|error| format!("admitted engine policy and bounds: {error:?}"))?;
    let service = runtime.epoch_service();
    let bytes = wat::parse_str(include_str!("../guest/actor.wat")).map_err(|error| error.to_string())?;
    assert_eq!(&bytes[..8], b"\0asm\x0d\0\x01\0", "core modules are not a fallback");
    // These facts describe only this embedded pilot fixture. Molten, not the
    // producer, must supply trusted Mantle materialization in a production host.
    let facts = materialization_facts(&bytes);
    let options = || hardened_wasmtime_shell::PrepareOptions {
        artifact: hardened_wasmtime_shell::Artifact {
            bytes: &bytes,
            facts: &facts,
        },
        descriptors: &[],
        deadline: hardened_wasmtime_runtime::epoch::deadline::Budget { budget_ticks: 1000 },
        epoch_service: Some(&service),
    };
    let mut missing_service = options();
    missing_service.epoch_service = None;
    assert!(matches!(
        hardened_wasmtime_shell::ComponentInvocation::prepare(&runtime, missing_service),
        Err(hardened_wasmtime_shell::PrepareError::DeadlineDenied)
    ));
    let invocation = hardened_wasmtime_shell::ComponentInvocation::prepare(&runtime, options())
        .map_err(|error| format!("exact actor component admission: {error:?}"))?;

    // Molten owns the epoch driver and its lifetime; the producer spawns none.
    // Join before checking outcomes so failures cannot strand a pilot thread.
    let stopped = std::sync::atomic::AtomicBool::new(false);
    let (completed, malformed, oversized) = std::thread::scope(|scope| {
        let driver = scope.spawn(|| {
            while !stopped.load(std::sync::atomic::Ordering::Acquire) {
                std::thread::park_timeout(std::time::Duration::from_millis(10));
                service.advance();
            }
        });
        let completed = invocation.run(EXPORT, &INPUT, (), |_, _| Err(hardened_wasmtime_shell::HostDenied));
        let malformed = invocation.run(EXPORT, &[7, 0, 0, 0], (), |_, _| Err(hardened_wasmtime_shell::HostDenied));
        let oversized = invocation.run(EXPORT, &[0; 9], (), |_, _| Err(hardened_wasmtime_shell::HostDenied));
        stopped.store(true, std::sync::atomic::Ordering::Release);
        driver.thread().unpark();
        driver.join().map_err(|_| String::from("consumer epoch driver panicked"))?;
        Ok::<_, String>((completed, malformed, oversized))
    })?;

    // Product actor semantics are checked here, never interpreted upstream.
    assert_eq!(completed.class, hardened_wasmtime_runtime::outcomes::Class::Completed);
    assert_eq!(completed.output, EXPECTED);
    assert!(completed.fuel_remaining > 0);
    assert!(completed.fuel_remaining < admitted.limits.fuel);
    assert_eq!(malformed.class, hardened_wasmtime_runtime::outcomes::Class::GuestDenied);
    assert!(malformed.output.is_empty());
    assert_eq!(oversized.class, hardened_wasmtime_runtime::outcomes::Class::InputLimitExceeded);
    assert!(oversized.output.is_empty());
    println!(
        "molten-hwr-consumer world=molten:component-runtime@1.0.0/actor export=actor-turn class=Completed actor=7 counter=42 output=070000002a000000 bytes=8"
    );
    println!("fuel_remaining={} fuel_budget={}", completed.fuel_remaining, admitted.limits.fuel);
    println!("denials malformed=GuestDenied oversized=InputLimitExceeded missing-epoch=DeadlineDenied");
    println!("scope=pilot-only production-adoption=false materialization-trust=false authority=false");
    Ok(())
}

fn materialization_facts(bytes: &[u8]) -> hardened_wasmtime_runtime::artifact::shape::MaterializationFacts {
    hardened_wasmtime_runtime::artifact::shape::MaterializationFacts {
        artifact_identity: blake3::hash(bytes).to_hex().to_string(),
        class: hardened_wasmtime_runtime::artifact::shape::Class::Component,
        world: hardened_wasmtime_runtime::artifact::shape::WorldIdentity {
            package: String::from("molten:component-runtime@1.0.0"),
            world: String::from("actor"),
        },
        imports: std::collections::BTreeSet::new(),
        exports: std::collections::BTreeSet::from([hardened_wasmtime_runtime::artifact::shape::InterfaceFunction {
            qualified_name: hardened_wasmtime_shell::qualified_name(&[EXPORT]),
            schema_identity: format!("blake3:{WIT_ID}"),
        }]),
    }
}
