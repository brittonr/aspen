fn reference(label: &str) -> String {
    molten::preserves_rail::content_ref_from_bytes(label.as_bytes())
}

#[test]
fn default_profile_preserves_original_native_fixture_selection() {
    let args = <super::super::Cli as clap::Parser>::try_parse_from([
        "cohort",
        "create",
        "--source-trust",
        "/trust",
        "--signed-output",
        "/output",
        "--issuer-key",
        "/issuer",
        "--holder-key",
        "/holder",
        "--policy",
        "/policy",
        "--output-parent",
        "/parent",
        "--name",
        "owned",
    ])
    .expect("default native CLI");
    let super::super::Command::Create(args) = args.command else {
        panic!("create command")
    };
    assert_eq!(args.profile, super::super::Profile::NativeFixture);
    assert_eq!(args.profile.executable(), "bin/molten-native-extension-fixture");
    assert!(super::selected_key(&args).expect("no actor selection").is_none());
    assert_eq!(super::super::Profile::Counter.executable(), "bin/molten-addressable-counter");
}

#[test]
fn cli_requires_explicit_namespace_type_key_and_placement() {
    let result = <super::super::Cli as clap::Parser>::try_parse_from([
        "cohort",
        "create",
        "--profile",
        "counter",
        "--source-trust",
        "/trust",
        "--signed-output",
        "/output",
        "--issuer-key",
        "/issuer",
        "--holder-key",
        "/holder",
        "--policy",
        "/policy",
        "--output-parent",
        "/parent",
        "--name",
        "owned",
    ]);
    assert!(result.is_err());
}

#[test]
fn live_time_selection_roundtrips_and_rejects_mutated_contract() {
    let admitted = molten::fabric_time::admit_profile(&super::profiles::time_descriptor(reference("time")))
        .expect("real time admission");
    let mut wire = molten::addressable_actor::command::configuration::TimeSelection::from_profile(&admitted);
    assert_eq!(wire.admit().expect("exact time selection").profile_ref, admitted.profile_ref);
    wire.max_timers += 1;
    assert!(wire.admit().is_err(), "numeric limits cannot change under a retained canonical profile ref");
}

#[test]
fn delivery_policy_is_valid_bounded_and_cannot_mix_retry_and_poison_classes() {
    let mut policy = super::profiles::delivery_policy(
        reference("complete"),
        reference("expire"),
        reference("redrive"),
        reference("retain"),
    );
    assert!(molten::core_api::coordination_delivery::validate_delivery_policy(&policy).is_ok());
    assert_eq!(policy.ready_capacity, 128);
    policy.poison_failure_classes.insert("transient".to_string());
    assert!(molten::core_api::coordination_delivery::validate_delivery_policy(&policy).is_err());
}

#[test]
fn exact_fixture_checkpoint_contains_key_and_completed_original_result() {
    let key = molten::core_api::addressable_actor::ActorKey {
        schema: molten::core_api::addressable_actor::ACTOR_KEY_SCHEMA.to_string(),
        namespace_ref: reference("namespace"),
        actor_type: "counter".to_string(),
        key: "tenant:one".to_string(),
    };
    let actor_key_ref = molten::core_api::addressable_actor::identify_actor_key(&key);
    let mut state = molten::addressable_actor::counter::State {
        actor_key_ref: actor_key_ref.clone(),
        value: 0,
        completed: Vec::new(),
    };
    let add = molten::addressable_actor::counter::Add {
        actor_key_ref,
        request_id: reference("request"),
        increment: 1,
    };
    assert_eq!(state.apply(&add).expect("increment"), 1);
    let checkpoint = state.materialize().expect("exact semantic bytes");
    let mut restored = molten::addressable_actor::counter::State::decode(&checkpoint.bytes).expect("checkpoint decode");
    assert_eq!(restored.apply(&add).expect("logical replay"), 1);
    assert_eq!(restored.materialize().expect("same semantic bytes").value_ref, checkpoint.value_ref);
    assert_eq!(restored.completed.len(), 1);
}
