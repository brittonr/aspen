pub(super) struct Inputs<'a> {
    pub configuration: &'a molten::addressable_actor::native::Configuration,
    pub placement: &'a molten::addressable_actor::command::placement::Selection,
    pub selected: &'a super::profiles::Selected,
    pub delivery: &'a molten::core_api::coordination_delivery::DeliveryManifest,
    pub bundle_ref: &'a str,
}

pub(super) fn publish(source: &super::super::source::Prepared, inputs: Inputs<'_>) -> molten::error::Result<String> {
    let Inputs {
        configuration,
        placement,
        selected,
        delivery,
        bundle_ref,
    } = inputs;
    let root = &source.root;
    let configuration_ref = json(root, "actor-configuration.json", configuration)?;
    json(root, "assignment-proposal.json", placement)?;
    json(root, "delivery-policy.json", &selected.policy)?;
    json(root, "delivery-manifest.json", delivery)?;
    json(
        root,
        "time-profile.json",
        &molten::addressable_actor::command::configuration::TimeSelection::from_profile(&selected.time),
    )?;
    json(
        root,
        "membership-profile.json",
        &molten::addressable_actor::command::configuration::MembershipSelection::from_profile(&selected.membership),
    )?;
    let key = molten::core_api::addressable_actor::identify_actor_key(&configuration.actor_key);
    let mut state = molten::addressable_actor::counter::State {
        actor_key_ref: key.clone(),
        value: 0,
        completed: Vec::new(),
    };
    let initial = state.materialize()?;
    value(root, "expected-initial-state.preserves", &initial)?;
    let checkpoint_refs = checkpoints(root, &key, &mut state)?;
    json(
        root,
        "flow-config.json",
        &serde_json::json!({
            "schema":"aspen-measured-counter-flow-v1", "bundle_ref":bundle_ref,
            "signed_output":source.output, "actor_key_ref":key,
            "expected_initial_ref":initial.value_ref, "expected_checkpoint_refs":checkpoint_refs,
            "expected_values":[1,3], "execution_evidence":"missing-until-the-executable-flow-is-run",
            "prerequisites":["independent-signed-source-output-and-source-gate-trust","external-UCAN-issuer-and-holder-keys",
                "operator-policy-for-every-exact-preview","actual-approved-placement-plan-reservation-fencing-and-authority",
                "Linux-x86-64-native-bounded-process-and-live-Redb-owners"],
            "non_claims":["not-production-admission","authoring-is-not-execution","private-seeds-not-retained","source-location-is-not-machine-loss-durability-evidence"]
        }),
    )?;
    root.write_new(&molten::node_state::RelativePath::parse("counter-flow.py")?, include_bytes!("flow.py"))?;
    root.write_new(&molten::node_state::RelativePath::parse("dialogue.py")?, include_bytes!("dialogue.py"))?;
    Ok(configuration_ref)
}

fn checkpoints(
    root: &molten::node_state::DirectoryRoot,
    key: &str,
    state: &mut molten::addressable_actor::counter::State,
) -> molten::error::Result<Vec<String>> {
    let mut checkpoint_refs = Vec::with_capacity(2);
    for (index, increment) in [1_u64, 2].into_iter().enumerate() {
        let request_id = super::super::source::record(root, "counter-pinned-logical-request-v1", vec![
            molten::preserves_rail::string(key),
            molten::preserves_rail::u64_value(index as u64 + 1),
            molten::preserves_rail::u64_value(increment),
        ])?;
        let add = molten::addressable_actor::counter::Add {
            actor_key_ref: key.to_string(),
            request_id,
            increment,
        };
        let payload = add.materialize()?;
        value(root, &format!("request-{}.preserves", index + 1), &payload)?;
        if index == 0 {
            let wrong_key = super::super::source::record(root, "counter-negative-target-not-selected-v1", vec![
                molten::preserves_rail::string(key),
                molten::preserves_rail::string("denied-target-only-not-another-actor-configuration"),
            ])?;
            let wrong = molten::addressable_actor::counter::Add {
                actor_key_ref: wrong_key,
                request_id: add.request_id.clone(),
                increment,
            };
            value(root, "request-wrong-key.preserves", &wrong.materialize()?)?;
        }
        let result = state.apply(&add)?;
        let expected = molten::preserves_rail::record("addressable-counter-result-v1", vec![
            molten::preserves_rail::string(key),
            molten::preserves_rail::string(&add.request_id),
            molten::preserves_rail::u64_value(result),
        ]);
        let bytes = molten::preserves_rail::canonical_bytes(&expected)?;
        root.write_new(
            &molten::node_state::RelativePath::parse(&format!("expected-result-{}.preserves", index + 1))?,
            &bytes,
        )?;
        let checkpoint = state.materialize()?;
        value(root, &format!("expected-checkpoint-{}.preserves", index + 1), &checkpoint)?;
        checkpoint_refs.push(checkpoint.value_ref);
    }
    Ok(checkpoint_refs)
}

fn value(
    root: &molten::node_state::DirectoryRoot,
    path: &str,
    value: &molten::system_extension::NativeCallbackValue,
) -> molten::error::Result<()> {
    molten::system_extension::admit_callback_value(value, 262_144)
        .map_err(|error| super::super::invalid(format!("counter fixture value denied: {error:?}")))?;
    root.write_new(&molten::node_state::RelativePath::parse(path)?, &value.bytes)?;
    let canonical = molten::preserves_rail::strict_canonical_decode(&value.bytes)?.value;
    super::super::source::retain(root, &canonical)?;
    Ok(())
}
fn json(
    root: &molten::node_state::DirectoryRoot,
    path: &str,
    value: &impl serde::Serialize,
) -> molten::error::Result<String> {
    let bytes = serde_json::to_vec(value).map_err(|error| super::super::invalid(error.to_string()))?;
    root.write_new(&molten::node_state::RelativePath::parse(path)?, &bytes)?;
    super::super::source::record(root, "counter-authored-json-selection-v1", vec![
        molten::preserves_rail::string(path),
        molten::preserves_rail::string(
            std::str::from_utf8(&bytes).map_err(|error| super::super::invalid(error.to_string()))?,
        ),
        molten::preserves_rail::string("exact-authoring-input-not-a-successful-runtime-observation"),
    ])
}
