use preserves::ValueImpl;

#[derive(Default)]
struct SuiteFixtures {
    budget: super::model::Budget,
    has_budget_fixture: bool,
    actors: Option<Vec<super::model::ActorDecl>>,
    has_actor_fixture: bool,
    capabilities: crate::runtime::CapabilityContext,
    has_capability_fixture: bool,
    policy: crate::runtime::AdmissionPolicy,
    has_policy_fixture: bool,
}

enum SuiteFieldStatus {
    Applied,
    Unknown,
}

impl SuiteFixtures {
    fn apply_field(&mut self, field: &preserves::Value<preserves::IOValue>) -> crate::error::Result<SuiteFieldStatus> {
        if super::fields::value_has_record_label(field, "budget-v1") {
            self.apply_budget(field)?;
            return Ok(SuiteFieldStatus::Applied);
        }
        if super::fields::value_has_record_label(field, "actor-registry-v1") {
            self.apply_actors(field)?;
            return Ok(SuiteFieldStatus::Applied);
        }
        if super::fields::value_has_record_label(field, "capabilities-v1") {
            self.apply_capabilities(field)?;
            return Ok(SuiteFieldStatus::Applied);
        }
        if super::fields::value_has_record_label(field, "policy-v1") {
            self.apply_policy(field)?;
            return Ok(SuiteFieldStatus::Applied);
        }
        Ok(SuiteFieldStatus::Unknown)
    }

    fn apply_budget(&mut self, field: &preserves::Value<preserves::IOValue>) -> crate::error::Result<()> {
        if self.has_budget_fixture {
            return Err(crate::error::Failure::invalid_harness("duplicate suite budget fixture"));
        }
        self.budget = super::allowance::parse_budget_limits(&crate::preserves_rail::value_to_iovalue(field))?;
        self.has_budget_fixture = true;
        Ok(())
    }

    fn apply_actors(&mut self, field: &preserves::Value<preserves::IOValue>) -> crate::error::Result<()> {
        if self.actors.is_some() {
            return Err(crate::error::Failure::invalid_harness("duplicate suite actor registry fixture"));
        }
        self.actors = Some(super::roster::parse_actor_registry(&crate::preserves_rail::value_to_iovalue(field))?);
        self.has_actor_fixture = true;
        Ok(())
    }

    fn apply_capabilities(&mut self, field: &preserves::Value<preserves::IOValue>) -> crate::error::Result<()> {
        if self.has_capability_fixture {
            return Err(crate::error::Failure::invalid_harness("duplicate suite capability fixture"));
        }
        self.capabilities = super::grants::parse_capabilities(&crate::preserves_rail::value_to_iovalue(field))?;
        self.has_capability_fixture = true;
        Ok(())
    }

    fn apply_policy(&mut self, field: &preserves::Value<preserves::IOValue>) -> crate::error::Result<()> {
        if self.has_policy_fixture {
            return Err(crate::error::Failure::invalid_harness("duplicate suite policy fixture"));
        }
        self.policy = super::rules::parse_policy(&crate::preserves_rail::value_to_iovalue(field))?;
        self.has_policy_fixture = true;
        Ok(())
    }
}

pub fn parse_suite(value: &preserves::IOValue) -> crate::error::Result<super::model::Suite> {
    let suite = value
        .collect_simple_record("harness-suite-v1", None)
        .ok_or_else(|| crate::error::Failure::invalid_harness("expected <harness-suite-v1 ...>"))?;
    let arity = suite.fields_iter().count();
    if !(4..=8).contains(&arity) {
        return Err(crate::error::Failure::invalid_harness(format!(
            "expected <harness-suite-v1 ...> with arity 4 through 8, got {arity}"
        )));
    }
    let schema = super::fields::required_string(&suite[0], "suite schema")?;
    if schema != crate::preserves_rail::HARNESS_SUITE_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported suite schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_SUITE_SCHEMA
        )));
    }
    let name = super::fields::required_string(&suite[1], "suite name")?;
    let seed = super::fields::required_u64(&suite[2], "suite seed")?;
    let (cursor, fixtures) = suite_fixtures(&suite, arity)?;
    let step_values = super::fields::required_sequence(&suite[cursor], "suite steps")?;
    let mut steps = Vec::with_capacity(step_values.len());
    for step in step_values.iter() {
        steps.push(super::decode::parse_step(&step)?);
    }
    let actors = fixtures.actors.unwrap_or_else(|| super::cast::infer_actor_registry(&steps));
    Ok(super::model::Suite {
        name,
        seed,
        budget: fixtures.budget,
        budget_explicit: fixtures.has_budget_fixture,
        actors,
        actors_explicit: fixtures.has_actor_fixture,
        capabilities: fixtures.capabilities,
        capabilities_explicit: fixtures.has_capability_fixture,
        policy: fixtures.policy,
        steps,
        source_value: value.clone(),
    })
}

fn suite_fixtures(
    suite: &preserves::Record<preserves::Value<preserves::IOValue>>,
    arity: usize,
) -> crate::error::Result<(usize, SuiteFixtures)> {
    let mut cursor = 3;
    let mut fixtures = SuiteFixtures::default();
    while cursor < arity - 1 {
        match fixtures.apply_field(&suite[cursor])? {
            SuiteFieldStatus::Applied => cursor += 1,
            SuiteFieldStatus::Unknown => {
                return Err(crate::error::Failure::invalid_harness(
                    "unexpected suite field before steps; expected optional budget, actor registry, capabilities, policy, then steps",
                ));
            }
        }
    }
    Ok((cursor, fixtures))
}

pub fn suite_ref(suite: &super::model::Suite) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&suite.source_value)
}

pub(super) fn actor_decl_for_primary_actor<'a>(
    suite: &'a super::model::Suite,
    actor: &str,
) -> crate::error::Result<&'a super::model::ActorDecl> {
    suite
        .actors
        .iter()
        .find(|decl| decl.id == actor)
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("actor {actor} missing from executor registry")))
}

pub(super) fn actor_kind_for_primary_actor<'a>(
    suite: &'a super::model::Suite,
    actor: &str,
) -> crate::error::Result<&'a super::model::ActorKind> {
    actor_decl_for_primary_actor(suite, actor).map(|decl| &decl.kind)
}

#[cfg(test)]
mod tests {
    #[test]
    fn suite_roundtrip_preserves_canonical_hash() {
        let suite = crate::preserves_rail::parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "roundtrip" 1 [<clock "actor">]>"#,
        )
        .expect("parse suite");
        let parsed = super::parse_suite(&suite).expect("parse suite schema");
        let rendered = crate::preserves_rail::to_text(&parsed.source_value).expect("render suite");
        let reparsed = crate::preserves_rail::parse_text(&rendered).expect("reparse rendered suite");
        assert_eq!(
            crate::preserves_rail::canonical_hash(&suite).unwrap(),
            crate::preserves_rail::canonical_hash(&reparsed).unwrap()
        );
    }
}
