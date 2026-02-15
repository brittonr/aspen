// SQL query response types.

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SqlCellValue {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(String),
}

impl SqlCellValue {
    pub fn to_display_string(&self) -> String {
        match self {
            SqlCellValue::Null => "(null)".to_string(),
            SqlCellValue::Integer(i) => i.to_string(),
            SqlCellValue::Real(f) => f.to_string(),
            SqlCellValue::Text(s) => s.clone(),
            SqlCellValue::Blob(b64) => format!("[blob: {}]", b64),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlResultResponse {
    pub success: bool,
    pub columns: Option<Vec<String>>,
    pub rows: Option<Vec<Vec<SqlCellValue>>>,
    pub row_count: Option<u32>,
    pub is_truncated: Option<bool>,
    pub execution_time_ms: Option<u64>,
    pub error: Option<String>,
}
