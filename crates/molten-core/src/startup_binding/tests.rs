use super::*;

fn reference(byte: char) -> string::String {
    format!("blake3:{}", byte.to_string().repeat(64))
}

fn binding() -> Binding {
    Binding {
        schema: SCHEMA.into(),
        producer_profile: PRODUCER_PROFILE.into(),
        executable_ref: reference('a'),
        source_ref: reference('b'),
        dependencies_ref: reference('c'),
        toolchain_ref: reference('d'),
        policy_ref: reference('e'),
        profile_ref: reference('f'),
        artifacts: REQUIRED_ROLES
            .into_iter()
            .map(|role| Artifact {
                role: role.into(),
                content_ref: reference('1'),
            })
            .collect(),
    }
}

fn selection() -> Selection {
    Selection {
        binding_ref: reference('2'),
        policy_ref: reference('e'),
        profile_ref: reference('f'),
    }
}

// r[verify molten.audit_f02.candidate_binding]
#[test]
fn admission_requires_authenticated_exact_executing_candidate() {
    let binding = binding();
    let selected = selection();
    let mut observed = Observation {
        binding_ref: &selected.binding_ref,
        executing_ref: &binding.executable_ref,
        authenticated_output: true,
        artifacts: &binding.artifacts,
    };
    assert_eq!(admit(&binding, &selected, &observed), Ok(()));
    observed.authenticated_output = false;
    assert_eq!(admit(&binding, &selected, &observed), Err(Issue::UnauthenticatedOutput));
    observed.authenticated_output = true;
    let wrong = reference('3');
    observed.executing_ref = &wrong;
    assert_eq!(admit(&binding, &selected, &observed), Err(Issue::WrongCandidate));
    observed.executing_ref = &binding.executable_ref;
    observed.binding_ref = &wrong;
    assert_eq!(admit(&binding, &selected, &observed), Err(Issue::StaleSelection));
}

fn observe<'a>(binding: &'a Binding, selected: &'a Selection, artifacts: &'a [Artifact]) -> Observation<'a> {
    Observation {
        binding_ref: &selected.binding_ref,
        executing_ref: &binding.executable_ref,
        authenticated_output: true,
        artifacts,
    }
}

// r[verify molten.audit_f02.validation]
#[test]
fn cohort_rejects_tamper_missing_and_duplicate_roles() {
    let binding = binding();
    let selected = selection();
    let mut actual = binding.artifacts.clone();
    actual[0].content_ref = reference('3');
    assert_eq!(admit(&binding, &selected, &observe(&binding, &selected, &actual)), Err(Issue::ArtifactMismatch));
    assert_eq!(
        admit(&binding, &selected, &observe(&binding, &selected, &actual[..3])),
        Err(Issue::IncompleteCohort)
    );
    actual = binding.artifacts.clone();
    actual[1].role = actual[0].role.clone();
    assert_eq!(admit(&binding, &selected, &observe(&binding, &selected, &actual)), Err(Issue::IncompleteCohort));
}

// r[verify molten.audit_f02.fixture_isolation]
#[test]
fn historical_and_fixture_producer_profiles_are_not_promoted() {
    let mut binding = binding();
    let selected = selection();
    let artifacts = binding.artifacts.clone();
    let executing_ref = binding.executable_ref.clone();
    let observed = Observation {
        binding_ref: &selected.binding_ref,
        executing_ref: &executing_ref,
        authenticated_output: true,
        artifacts: &artifacts,
    };
    binding.schema = "node-startup-receipt-v1".into();
    assert_eq!(admit(&binding, &selected, &observed), Err(Issue::UnsupportedSchema));
    binding.schema = SCHEMA.into();
    binding.producer_profile = "explicit-test-fixture".into();
    assert_eq!(admit(&binding, &selected, &observed), Err(Issue::UnsupportedProducer));
    binding.producer_profile = PRODUCER_PROFILE.into();
    binding.policy_ref = reference('4');
    assert_eq!(admit(&binding, &selected, &observed), Err(Issue::PolicyMismatch));
    binding.policy_ref = selected.policy_ref.clone();
    binding.profile_ref = reference('5');
    assert_eq!(admit(&binding, &selected, &observed), Err(Issue::ProfileMismatch));
}
