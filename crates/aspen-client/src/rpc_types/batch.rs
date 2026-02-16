// Batch operation types.

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BatchWriteOperation {
    Set { key: String, value: Vec<u8> },
    Delete { key: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BatchCondition {
    ValueEquals { key: String, expected: Vec<u8> },
    KeyExists { key: String },
    KeyNotExists { key: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchReadResultResponse {
    #[serde(rename = "success")]
    pub is_success: bool,
    pub values: Option<Vec<Option<Vec<u8>>>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchWriteResultResponse {
    #[serde(rename = "success")]
    pub is_success: bool,
    pub operations_applied: Option<u32>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalBatchWriteResultResponse {
    #[serde(rename = "success")]
    pub is_success: bool,
    #[serde(rename = "conditions_met")]
    pub were_conditions_met: bool,
    pub operations_applied: Option<u32>,
    pub failed_condition_index: Option<u32>,
    pub failed_condition_reason: Option<String>,
    pub error: Option<String>,
}
