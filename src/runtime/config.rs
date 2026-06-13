use serde::Deserialize;
use serde::Serialize;

use super::ActorId;
use super::RuntimeValue;
use crate::error::MoltenError;
use crate::error::Result;

const MAX_RUNTIME_ACTORS: usize = 1024;
const MAX_RUNTIME_SUBSCRIPTIONS: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeStartupConfig {
    pub source_language: RuntimeConfigSource,
    pub actors: Vec<RuntimeActorConfig>,
    pub subscriptions: Vec<RuntimeSubscriptionConfig>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeConfigSource {
    Nickel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeActorConfig {
    pub id: ActorId,
    pub kind: RuntimeActorKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeActorKind {
    Native,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeSubscriptionConfig {
    pub actor: ActorId,
    pub subject_preserves: String,
}

impl RuntimeStartupConfig {
    pub fn from_nickel_export_json(source: &str) -> Result<Self> {
        let config: Self = serde_json::from_str(source)
            .map_err(|error| MoltenError::invalid_harness(format!("invalid Nickel runtime export JSON: {error}")))?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        if self.source_language != RuntimeConfigSource::Nickel {
            return Err(MoltenError::invalid_harness("runtime startup config must come from Nickel export"));
        }
        if self.actors.len() > MAX_RUNTIME_ACTORS {
            return Err(MoltenError::invalid_harness(format!(
                "runtime startup config exceeds {MAX_RUNTIME_ACTORS} actors"
            )));
        }
        if self.subscriptions.len() > MAX_RUNTIME_SUBSCRIPTIONS {
            return Err(MoltenError::invalid_harness(format!(
                "runtime startup config exceeds {MAX_RUNTIME_SUBSCRIPTIONS} subscriptions"
            )));
        }
        for subscription in &self.subscriptions {
            RuntimeValue::new(crate::preserves_rail::parse_text(&subscription.subject_preserves)?)?;
            if !self.actors.iter().any(|actor| actor.id == subscription.actor) {
                return Err(MoltenError::invalid_harness(format!(
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
    use super::RuntimeActorKind;
    use super::RuntimeConfigSource;
    use super::RuntimeStartupConfig;

    #[test]
    fn nickel_export_loads_typed_actor_and_subscription_config() {
        let source = r#"{
            "source_language": "nickel",
            "actors": [{ "id": "actor:consumer", "kind": "native" }],
            "subscriptions": [{ "actor": "actor:consumer", "subject_preserves": "\"service.ready\"" }]
        }"#;
        let config = RuntimeStartupConfig::from_nickel_export_json(source).expect("config");
        assert_eq!(config.source_language, RuntimeConfigSource::Nickel);
        assert_eq!(config.actors[0].kind, RuntimeActorKind::Native);
        assert_eq!(config.actors[0].id.as_str(), "actor:consumer");
        assert_eq!(config.subscriptions[0].subject_preserves, "\"service.ready\"");
    }

    #[test]
    fn nickel_export_rejects_subscription_for_undeclared_actor() {
        let source = r#"{
            "source_language": "nickel",
            "actors": [],
            "subscriptions": [{ "actor": "actor:missing", "subject_preserves": "\"service.ready\"" }]
        }"#;
        let error = RuntimeStartupConfig::from_nickel_export_json(source).expect_err("missing actor");
        assert!(error.to_string().contains("subscription actor actor:missing is not declared"));
    }
}
