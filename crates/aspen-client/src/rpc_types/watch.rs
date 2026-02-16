// Watch operation response types.

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchCreateResultResponse {
    pub success: bool,
    pub watch_id: Option<u64>,
    pub current_index: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchCancelResultResponse {
    pub success: bool,
    pub watch_id: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchStatusResultResponse {
    pub success: bool,
    pub watches: Option<Vec<WatchInfo>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchInfo {
    pub watch_id: u64,
    pub prefix: String,
    pub last_sent_index: u64,
    pub events_sent: u64,
    pub created_at_ms: u64,
    pub should_include_prev_value: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchEventResponse {
    pub watch_id: u64,
    pub index: u64,
    pub term: u64,
    pub committed_at_ms: u64,
    pub events: Vec<WatchKeyEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchKeyEvent {
    pub event_type: WatchEventType,
    pub key: String,
    pub value: Option<Vec<u8>>,
    pub prev_value: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WatchEventType {
    Put,
    Delete,
}
