const MAX_RUNTIME_ACTORS: usize = 1024;
const MAX_RUNTIME_SUBSCRIPTIONS: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RuntimeStartupConfig {
    pub source_language: RuntimeConfigSource,
    #[serde(default)]
    pub consensus: crate::raft_control_plane::ClusterConsensusConfig,
    pub actors: Vec<RuntimeActorConfig>,
    pub subscriptions: Vec<RuntimeSubscriptionConfig>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeConfigSource {
    Nickel,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RuntimeActorConfig {
    pub id: super::ActorId,
    pub kind: RuntimeActorKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeActorKind {
    Native,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RuntimeSubscriptionConfig {
    pub actor: super::ActorId,
    pub subject_preserves: String,
}

impl RuntimeStartupConfig {
    pub fn from_nickel_export_json(source: &str) -> crate::error::Result<Self> {
        let config: Self = serde_json::from_str(source).map_err(|error| {
            crate::error::MoltenError::invalid_harness(format!("invalid Nickel runtime export JSON: {error}"))
        })?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> crate::error::Result<()> {
        if self.source_language != RuntimeConfigSource::Nickel {
            return Err(crate::error::MoltenError::invalid_harness(
                "runtime startup config must come from Nickel export",
            ));
        }
        crate::raft_control_plane::validate_cluster_consensus_config(&self.consensus)?;
        if self.actors.len() > MAX_RUNTIME_ACTORS {
            return Err(crate::error::MoltenError::invalid_harness(format!(
                "runtime startup config exceeds {MAX_RUNTIME_ACTORS} actors"
            )));
        }
        if self.subscriptions.len() > MAX_RUNTIME_SUBSCRIPTIONS {
            return Err(crate::error::MoltenError::invalid_harness(format!(
                "runtime startup config exceeds {MAX_RUNTIME_SUBSCRIPTIONS} subscriptions"
            )));
        }
        for subscription in &self.subscriptions {
            super::RuntimeValue::new(crate::preserves_rail::parse_text(&subscription.subject_preserves)?)?;
            if !self.actors.iter().any(|actor| actor.id == subscription.actor) {
                return Err(crate::error::MoltenError::invalid_harness(format!(
                    "subscription actor {} is not declared",
                    subscription.actor.as_str()
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    const TEST_CONTENT_REF: &str = "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn nickel_export_loads_typed_actor_and_subscription_config() {
        let source = r#"{
            "source_language": "nickel",
            "actors": [{ "id": "actor:consumer", "kind": "native" }],
            "subscriptions": [{ "actor": "actor:consumer", "subject_preserves": "\"service.ready\"" }]
        }"#;
        let config = super::RuntimeStartupConfig::from_nickel_export_json(source).expect("config");
        assert_eq!(config.source_language, super::RuntimeConfigSource::Nickel);
        assert_eq!(config.consensus.algorithm_profile, crate::raft_control_plane::CONSENSUS_PROFILE_RAFT);
        assert_eq!(config.actors[0].kind, super::RuntimeActorKind::Native);
        assert_eq!(config.actors[0].id.as_str(), "actor:consumer");
        assert_eq!(config.subscriptions[0].subject_preserves, "\"service.ready\"");
    }

    #[test]
    fn nickel_export_loads_explicit_consensus_profile_config() {
        let source = format!(
            r#"{{
            "source_language": "nickel",
            "consensus": {{
                "algorithm_profile": "raft",
                "profile_version": "raft-production-v1",
                "placement_ref": "{TEST_CONTENT_REF}",
                "required_evidence_refs": ["{TEST_CONTENT_REF}"]
            }},
            "actors": [{{ "id": "actor:consumer", "kind": "native" }}],
            "subscriptions": []
        }}"#
        );
        let config = super::RuntimeStartupConfig::from_nickel_export_json(&source).expect("config");
        assert_eq!(config.consensus.algorithm_profile, crate::raft_control_plane::CONSENSUS_PROFILE_RAFT);
        assert_eq!(config.consensus.profile_version.as_deref(), Some("raft-production-v1"));
        assert_eq!(config.consensus.placement_ref.as_deref(), Some(TEST_CONTENT_REF));
    }

    #[test]
    fn nickel_export_rejects_unknown_consensus_profile_config() {
        let source = r#"{
            "source_language": "nickel",
            "consensus": { "algorithm_profile": "raftt" },
            "actors": [],
            "subscriptions": []
        }"#;
        let error = super::RuntimeStartupConfig::from_nickel_export_json(source).expect_err("bad consensus");
        assert!(error.to_string().contains("unsupported consensus algorithm profile raftt"));
    }

    #[test]
    fn nickel_export_rejects_subscription_for_undeclared_actor() {
        let source = r#"{
            "source_language": "nickel",
            "actors": [],
            "subscriptions": [{ "actor": "actor:missing", "subject_preserves": "\"service.ready\"" }]
        }"#;
        let error = super::RuntimeStartupConfig::from_nickel_export_json(source).expect_err("missing actor");
        assert!(error.to_string().contains("subscription actor actor:missing is not declared"));
    }
}
