//! Index field type definitions and index metadata.

use serde::Deserialize;
use serde::Serialize;

use super::INDEX_METADATA_PREFIX;

/// Serializable index definition for persistence.
///
/// This struct captures the metadata needed to recreate an index, including
/// its name, type, and extraction field. It can be stored in the system
/// namespace and loaded on startup to restore custom indexes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndexDefinition {
    /// Unique name for this index (e.g., "idx_mod_revision").
    pub name: String,

    /// Type of field being indexed.
    pub field_type: IndexFieldType,

    /// Name of the built-in field to index (for builtin types).
    /// Options: "mod_revision", "create_revision", "expires_at_ms", "lease_id"
    pub field: Option<String>,

    /// Whether this is a built-in index that's always present.
    pub builtin: bool,

    /// Index options.
    #[serde(default)]
    pub options: IndexOptions,
}

/// Type of the indexed field.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum IndexFieldType {
    /// 64-bit integer field (mod_revision, create_revision, etc.)
    #[default]
    Integer,
    /// String field from value (future: JSON path extraction)
    String,
    /// Unsigned 64-bit integer (expires_at_ms)
    UnsignedInteger,
}

/// Additional index configuration options.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct IndexOptions {
    /// Whether null values should be indexed.
    #[serde(default)]
    pub index_nulls: bool,

    /// Whether this index enforces uniqueness.
    #[serde(default)]
    pub unique: bool,
}

impl IndexDefinition {
    /// Create a new index definition for a built-in field.
    pub fn builtin(name: &str, field: &str, field_type: IndexFieldType) -> Self {
        Self {
            name: name.to_string(),
            field_type,
            field: Some(field.to_string()),
            builtin: true,
            options: IndexOptions::default(),
        }
    }

    /// Create a new custom index definition.
    pub fn custom(name: &str, field: &str, field_type: IndexFieldType) -> Self {
        Self {
            name: name.to_string(),
            field_type,
            field: Some(field.to_string()),
            builtin: false,
            options: IndexOptions::default(),
        }
    }

    /// Set the unique option.
    pub fn with_unique(mut self, unique: bool) -> Self {
        self.options.unique = unique;
        self
    }

    /// Set the index_nulls option.
    pub fn with_index_nulls(mut self, index_nulls: bool) -> Self {
        self.options.index_nulls = index_nulls;
        self
    }

    /// Get the system key for storing this index definition.
    pub fn system_key(&self) -> String {
        format!("{}{}", INDEX_METADATA_PREFIX, self.name)
    }

    /// Create builtin index definitions.
    pub fn builtins() -> Vec<Self> {
        vec![
            Self::builtin("idx_mod_revision", "mod_revision", IndexFieldType::Integer),
            Self::builtin("idx_create_revision", "create_revision", IndexFieldType::Integer),
            Self::builtin("idx_expires_at", "expires_at_ms", IndexFieldType::UnsignedInteger),
            Self::builtin("idx_lease_id", "lease_id", IndexFieldType::UnsignedInteger),
        ]
    }
}
