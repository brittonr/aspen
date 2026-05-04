//! Structured evidence receipts for dogfood runs.
//!
//! The shape intentionally mirrors the release-evidence discipline used by
//! Crunch: explicit schema constants, bounded collections, deterministic JSON,
//! and validation before treating a receipt as durable evidence.

use std::error::Error;
use std::fmt;

use serde::Deserialize;
use serde::Serialize;

pub const DOGFOOD_RUN_RECEIPT_SCHEMA: &str = "aspen.dogfood.run-receipt.v1";
pub const MAX_RECEIPT_STAGES: usize = 16;
pub const MAX_STAGE_ARTIFACTS: usize = 32;

/// Dogfood receipt stage metadata used as the source of truth for acceptance
/// lifecycle docs and tests.
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DogfoodStageContract {
    pub stage: DogfoodStageKind,
    pub name: &'static str,
    pub default_full_required: bool,
    pub leave_running_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DogfoodRunReceipt {
    pub schema: String,
    pub run_id: String,
    pub command: String,
    pub created_at: String,
    pub mode: DogfoodRunMode,
    pub project_dir: String,
    pub cluster_dir: String,
    pub stages: Vec<DogfoodStageReceipt>,
}

impl DogfoodRunReceipt {
    pub fn new(init: DogfoodRunReceiptInit) -> Self {
        Self {
            schema: DOGFOOD_RUN_RECEIPT_SCHEMA.to_string(),
            run_id: init.run_id,
            command: init.command,
            created_at: init.created_at,
            mode: init.mode,
            project_dir: init.project_dir,
            cluster_dir: init.cluster_dir,
            stages: init.stages,
        }
    }

    pub fn validate(&self) -> Result<(), ReceiptValidationError> {
        validate_required("schema", &self.schema)?;
        if self.schema != DOGFOOD_RUN_RECEIPT_SCHEMA {
            return Err(ReceiptValidationError::UnexpectedSchema {
                actual: self.schema.clone(),
                expected: DOGFOOD_RUN_RECEIPT_SCHEMA,
            });
        }
        validate_required("run_id", &self.run_id)?;
        validate_required("command", &self.command)?;
        validate_required("created_at", &self.created_at)?;
        validate_required("project_dir", &self.project_dir)?;
        validate_required("cluster_dir", &self.cluster_dir)?;
        if self.stages.len() > MAX_RECEIPT_STAGES {
            return Err(ReceiptValidationError::TooManyStages {
                count: self.stages.len(),
                max: MAX_RECEIPT_STAGES,
            });
        }
        for stage in &self.stages {
            stage.validate()?;
        }
        Ok(())
    }

    pub fn canonical_json_bytes(&self) -> Result<Vec<u8>, ReceiptJsonError> {
        self.validate().map_err(ReceiptJsonError::Validation)?;
        serde_json::to_vec(self).map_err(ReceiptJsonError::Serialize)
    }

    pub fn write_canonical_json_file(&self, path: &std::path::Path) -> Result<(), ReceiptJsonError> {
        let bytes = self.canonical_json_bytes()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(ReceiptJsonError::Io)?;
        }
        std::fs::write(path, bytes).map_err(ReceiptJsonError::Io)
    }

    pub fn from_canonical_json_bytes(bytes: &[u8]) -> Result<Self, ReceiptJsonError> {
        let receipt: Self = serde_json::from_slice(bytes).map_err(ReceiptJsonError::Deserialize)?;
        receipt.validate().map_err(ReceiptJsonError::Validation)?;
        Ok(receipt)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DogfoodRunReceiptInit {
    pub run_id: String,
    pub command: String,
    pub created_at: String,
    pub mode: DogfoodRunMode,
    pub project_dir: String,
    pub cluster_dir: String,
    pub stages: Vec<DogfoodStageReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DogfoodRunMode {
    pub federation: bool,
    pub vm_ci: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DogfoodStageReceipt {
    pub stage: DogfoodStageKind,
    pub status: DogfoodStageStatus,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub failure: Option<DogfoodFailureSummary>,
    pub artifacts: Vec<DogfoodArtifactReceipt>,
}

impl DogfoodStageReceipt {
    pub fn validate(&self) -> Result<(), ReceiptValidationError> {
        validate_required("stage.started_at", &self.started_at)?;
        match (&self.status, &self.failure) {
            (DogfoodStageStatus::Failed, None) => {
                return Err(ReceiptValidationError::MissingFailure { stage: self.stage });
            }
            (DogfoodStageStatus::Succeeded, Some(_)) => {
                return Err(ReceiptValidationError::UnexpectedFailure { stage: self.stage });
            }
            _ => {}
        }
        if let Some(failure) = &self.failure {
            failure.validate()?;
        }
        if self.artifacts.len() > MAX_STAGE_ARTIFACTS {
            return Err(ReceiptValidationError::TooManyArtifacts {
                stage: self.stage,
                count: self.artifacts.len(),
                max: MAX_STAGE_ARTIFACTS,
            });
        }
        for artifact in &self.artifacts {
            artifact.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DogfoodStageKind {
    Start,
    Push,
    Build,
    Deploy,
    Verify,
    PublishReceipt,
    Stop,
}

impl DogfoodStageKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Push => "push",
            Self::Build => "build",
            Self::Deploy => "deploy",
            Self::Verify => "verify",
            Self::PublishReceipt => "publish_receipt",
            Self::Stop => "stop",
        }
    }
}

/// Ordered dogfood full-run stage contract.
///
/// `full --leave-running` intentionally omits `stop` after successful receipt
/// publication so operators can inspect the live cluster. All earlier stages are
/// required in both success modes.
#[cfg(test)]
pub const DOGFOOD_FULL_STAGE_CONTRACTS: &[DogfoodStageContract] = &[
    DogfoodStageContract {
        stage: DogfoodStageKind::Start,
        name: "start",
        default_full_required: true,
        leave_running_required: true,
    },
    DogfoodStageContract {
        stage: DogfoodStageKind::Push,
        name: "push",
        default_full_required: true,
        leave_running_required: true,
    },
    DogfoodStageContract {
        stage: DogfoodStageKind::Build,
        name: "build",
        default_full_required: true,
        leave_running_required: true,
    },
    DogfoodStageContract {
        stage: DogfoodStageKind::Deploy,
        name: "deploy",
        default_full_required: true,
        leave_running_required: true,
    },
    DogfoodStageContract {
        stage: DogfoodStageKind::Verify,
        name: "verify",
        default_full_required: true,
        leave_running_required: true,
    },
    DogfoodStageContract {
        stage: DogfoodStageKind::PublishReceipt,
        name: "publish_receipt",
        default_full_required: true,
        leave_running_required: true,
    },
    DogfoodStageContract {
        stage: DogfoodStageKind::Stop,
        name: "stop",
        default_full_required: true,
        leave_running_required: false,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DogfoodStageStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Skipped,
}

impl DogfoodStageStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DogfoodFailureSummary {
    pub operation: String,
    pub category: String,
    pub message: String,
}

impl DogfoodFailureSummary {
    pub fn validate(&self) -> Result<(), ReceiptValidationError> {
        validate_required("failure.operation", &self.operation)?;
        validate_required("failure.category", &self.category)?;
        validate_required("failure.message", &self.message)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DogfoodArtifactReceipt {
    pub name: String,
    pub kind: DogfoodArtifactKind,
    pub store_id: Option<String>,
    pub blob_id: Option<String>,
    pub digest: Option<String>,
    pub size_bytes: Option<u64>,
    pub relative_path: Option<String>,
}

impl DogfoodArtifactReceipt {
    pub fn validate(&self) -> Result<(), ReceiptValidationError> {
        validate_required("artifact.name", &self.name)?;
        if let Some(relative_path) = &self.relative_path {
            validate_required("artifact.relative_path", relative_path)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DogfoodArtifactKind {
    ClusterTicket,
    ForgeRepository,
    GitCommit,
    CiRun,
    CiArtifact,
    Deployment,
    VerificationReport,
    Receipt,
    LogExcerpt,
}

impl DogfoodArtifactKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ClusterTicket => "cluster_ticket",
            Self::ForgeRepository => "forge_repository",
            Self::GitCommit => "git_commit",
            Self::CiRun => "ci_run",
            Self::CiArtifact => "ci_artifact",
            Self::Deployment => "deployment",
            Self::VerificationReport => "verification_report",
            Self::Receipt => "receipt",
            Self::LogExcerpt => "log_excerpt",
        }
    }
}

#[derive(Debug)]
pub enum ReceiptJsonError {
    Serialize(serde_json::Error),
    Deserialize(serde_json::Error),
    Validation(ReceiptValidationError),
    Io(std::io::Error),
}

impl fmt::Display for ReceiptJsonError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Serialize(error) => write!(formatter, "serializing dogfood receipt: {error}"),
            Self::Deserialize(error) => write!(formatter, "parsing dogfood receipt: {error}"),
            Self::Validation(error) => write!(formatter, "validating dogfood receipt: {error}"),
            Self::Io(error) => write!(formatter, "writing dogfood receipt: {error}"),
        }
    }
}

impl Error for ReceiptJsonError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Serialize(error) | Self::Deserialize(error) => Some(error),
            Self::Validation(error) => Some(error),
            Self::Io(error) => Some(error),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceiptValidationError {
    EmptyField {
        field: &'static str,
    },
    UnexpectedSchema {
        actual: String,
        expected: &'static str,
    },
    TooManyStages {
        count: usize,
        max: usize,
    },
    TooManyArtifacts {
        stage: DogfoodStageKind,
        count: usize,
        max: usize,
    },
    MissingFailure {
        stage: DogfoodStageKind,
    },
    UnexpectedFailure {
        stage: DogfoodStageKind,
    },
}

impl fmt::Display for ReceiptValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyField { field } => write!(formatter, "{field} must not be empty"),
            Self::UnexpectedSchema { actual, expected } => {
                write!(formatter, "receipt schema {actual} does not match expected {expected}")
            }
            Self::TooManyStages { count, max } => {
                write!(formatter, "receipt has {count} stages, limit is {max}")
            }
            Self::TooManyArtifacts { stage, count, max } => {
                write!(formatter, "{stage:?} stage has {count} artifacts, limit is {max}")
            }
            Self::MissingFailure { stage } => write!(formatter, "{stage:?} stage failed without failure detail"),
            Self::UnexpectedFailure { stage } => write!(formatter, "{stage:?} stage succeeded with failure detail"),
        }
    }
}

impl Error for ReceiptValidationError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), ReceiptValidationError> {
    if value.trim().is_empty() {
        return Err(ReceiptValidationError::EmptyField { field });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_receipt() -> DogfoodRunReceipt {
        DogfoodRunReceipt::new(DogfoodRunReceiptInit {
            run_id: "dogfood-20260503T040709Z".to_string(),
            command: "full".to_string(),
            created_at: "2026-05-03T04:07:09Z".to_string(),
            mode: DogfoodRunMode {
                federation: false,
                vm_ci: true,
            },
            project_dir: "/home/brittonr/git/aspen".to_string(),
            cluster_dir: "/tmp/aspen-dogfood".to_string(),
            stages: vec![DogfoodStageReceipt {
                stage: DogfoodStageKind::Build,
                status: DogfoodStageStatus::Succeeded,
                started_at: "2026-05-03T04:08:00Z".to_string(),
                finished_at: Some("2026-05-03T04:09:00Z".to_string()),
                failure: None,
                artifacts: vec![DogfoodArtifactReceipt {
                    name: "aspen-node".to_string(),
                    kind: DogfoodArtifactKind::CiArtifact,
                    store_id: Some("/nix/store/example-aspen-node".to_string()),
                    blob_id: Some("bafk-example".to_string()),
                    digest: Some("blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
                    size_bytes: Some(42),
                    relative_path: Some("artifacts/aspen-node".to_string()),
                }],
            }],
        })
    }

    #[test]
    fn receipt_serializes_with_stable_schema_and_fields() {
        let receipt = sample_receipt();
        let json = String::from_utf8(receipt.canonical_json_bytes().unwrap()).unwrap();

        assert!(json.contains("\"schema\":\"aspen.dogfood.run-receipt.v1\""));
        assert!(json.contains("\"run_id\":\"dogfood-20260503T040709Z\""));
        assert!(json.contains("\"command\":\"full\""));
        assert!(json.contains("\"stages\""));
        assert!(json.contains("\"artifacts\""));
        assert!(json.contains("\"status\":\"succeeded\""));
    }

    #[test]
    fn receipt_json_round_trips() {
        let receipt = sample_receipt();
        let bytes = receipt.canonical_json_bytes().unwrap();
        let parsed = DogfoodRunReceipt::from_canonical_json_bytes(&bytes).unwrap();

        assert_eq!(receipt, parsed);
    }

    #[test]
    fn failed_stage_requires_failure_summary() {
        let mut receipt = sample_receipt();
        receipt.stages[0].status = DogfoodStageStatus::Failed;

        let error = receipt.validate().unwrap_err();
        assert_eq!(error, ReceiptValidationError::MissingFailure {
            stage: DogfoodStageKind::Build,
        });
    }

    #[test]
    fn succeeded_stage_rejects_failure_summary() {
        let mut receipt = sample_receipt();
        receipt.stages[0].failure = Some(DogfoodFailureSummary {
            operation: "ci wait".to_string(),
            category: "timeout".to_string(),
            message: "timed out".to_string(),
        });

        let error = receipt.validate().unwrap_err();
        assert_eq!(error, ReceiptValidationError::UnexpectedFailure {
            stage: DogfoodStageKind::Build,
        });
    }

    #[test]
    fn receipt_rejects_empty_run_id() {
        let mut receipt = sample_receipt();
        receipt.run_id = " ".to_string();

        let error = receipt.validate().unwrap_err();
        assert_eq!(error, ReceiptValidationError::EmptyField { field: "run_id" });
    }

    #[test]
    fn receipt_rejects_unexpected_schema() {
        let mut receipt = sample_receipt();
        receipt.schema = "aspen.dogfood.run-receipt.v0".to_string();

        let error = receipt.validate().unwrap_err();
        assert_eq!(error, ReceiptValidationError::UnexpectedSchema {
            actual: "aspen.dogfood.run-receipt.v0".to_string(),
            expected: DOGFOOD_RUN_RECEIPT_SCHEMA,
        });
    }

    #[test]
    fn receipt_rejects_too_many_stages() {
        let mut receipt = sample_receipt();
        receipt.stages = vec![receipt.stages[0].clone(); MAX_RECEIPT_STAGES + 1];

        let error = receipt.validate().unwrap_err();
        assert_eq!(error, ReceiptValidationError::TooManyStages {
            count: MAX_RECEIPT_STAGES + 1,
            max: MAX_RECEIPT_STAGES,
        });
    }

    #[test]
    fn stage_rejects_too_many_artifacts() {
        let mut receipt = sample_receipt();
        let artifact = receipt.stages[0].artifacts[0].clone();
        receipt.stages[0].artifacts = vec![artifact; MAX_STAGE_ARTIFACTS + 1];

        let error = receipt.validate().unwrap_err();
        assert_eq!(error, ReceiptValidationError::TooManyArtifacts {
            stage: DogfoodStageKind::Build,
            count: MAX_STAGE_ARTIFACTS + 1,
            max: MAX_STAGE_ARTIFACTS,
        });
    }

    #[test]
    fn dogfood_full_stage_contract_matches_stage_names() {
        let expected = [
            DogfoodStageKind::Start,
            DogfoodStageKind::Push,
            DogfoodStageKind::Build,
            DogfoodStageKind::Deploy,
            DogfoodStageKind::Verify,
            DogfoodStageKind::PublishReceipt,
            DogfoodStageKind::Stop,
        ];

        assert_eq!(DOGFOOD_FULL_STAGE_CONTRACTS.len(), expected.len());

        for (contract, expected_stage) in DOGFOOD_FULL_STAGE_CONTRACTS.iter().zip(expected) {
            assert_eq!(contract.stage, expected_stage);
            assert_eq!(contract.name, expected_stage.as_str());
            assert!(contract.default_full_required, "default full requires `{}`", contract.name);
        }

        let leave_running_stages: Vec<&str> = DOGFOOD_FULL_STAGE_CONTRACTS
            .iter()
            .filter(|contract| contract.leave_running_required)
            .map(|contract| contract.name)
            .collect();

        assert_eq!(leave_running_stages, ["start", "push", "build", "deploy", "verify", "publish_receipt"]);
    }

    #[test]
    fn dogfood_full_stage_contract_is_documented() {
        let deploy_doc = include_str!("../../../docs/deploy.md");

        assert!(deploy_doc.contains("DOGFOOD_FULL_STAGE_CONTRACTS"));

        for contract in DOGFOOD_FULL_STAGE_CONTRACTS {
            let row_prefix = format!("| `{}` |", contract.name);
            let row = deploy_doc
                .lines()
                .find(|line| line.starts_with(&row_prefix))
                .unwrap_or_else(|| panic!("deploy docs must include `{}` dogfood stage row", contract.name));

            let columns: Vec<&str> = row.split('|').map(str::trim).collect();
            assert_eq!(
                columns.get(2),
                Some(&if contract.default_full_required {
                    "required"
                } else {
                    "omitted"
                })
            );
            assert_eq!(
                columns.get(3),
                Some(&if contract.leave_running_required {
                    "required"
                } else {
                    "omitted"
                })
            );
        }
    }
}
