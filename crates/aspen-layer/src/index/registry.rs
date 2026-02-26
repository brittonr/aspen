//! Index registry for managing multiple secondary indexes.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::MAX_INDEXES;
use super::errors::IndexError;
use super::errors::IndexResult;
use super::field_types::IndexDefinition;
use super::secondary_index::SecondaryIndex;
use crate::subspace::Subspace;
use crate::tuple::Tuple;

/// Registry for managing multiple secondary indexes.
///
/// The registry provides a centralized place to define and access indexes,
/// and generates the operations needed to update all indexes during writes.
pub struct IndexRegistry {
    /// Registered indexes by name.
    indexes: BTreeMap<String, SecondaryIndex>,
}

impl IndexRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            indexes: BTreeMap::new(),
        }
    }

    /// Create a registry with the built-in indexes.
    ///
    /// Built-in indexes:
    /// - `idx_mod_revision`: Query by modification revision
    /// - `idx_create_revision`: Query by creation revision
    /// - `idx_expires_at`: Query by expiration time (for TTL cleanup)
    /// - `idx_lease_id`: Query keys by lease
    ///
    /// Note: Registration of builtin indexes cannot fail because MAX_INDEXES > 4
    /// and the registry starts empty. Errors are silently ignored for Tiger Style
    /// compliance (no panics in initialization code).
    pub fn with_builtins(index_subspace: Subspace) -> Self {
        let mut registry = Self::new();

        // idx_mod_revision: Query by modification revision
        // Note: Registration cannot fail as we're adding to an empty registry with capacity > 4
        let mod_rev_space = index_subspace.subspace(&Tuple::new().push("mod_revision"));
        let _ = registry.register(SecondaryIndex::numeric(
            "idx_mod_revision",
            mod_rev_space,
            Arc::new(|entry| Some(entry.mod_revision.to_be_bytes().to_vec())),
        ));

        // idx_create_revision: Query by creation revision
        let create_rev_space = index_subspace.subspace(&Tuple::new().push("create_revision"));
        let _ = registry.register(SecondaryIndex::numeric(
            "idx_create_revision",
            create_rev_space,
            Arc::new(|entry| Some(entry.create_revision.to_be_bytes().to_vec())),
        ));

        // idx_expires_at: Query by expiration time (for TTL cleanup)
        let expires_space = index_subspace.subspace(&Tuple::new().push("expires_at"));
        let _ = registry.register(SecondaryIndex::numeric(
            "idx_expires_at",
            expires_space,
            Arc::new(|entry| entry.expires_at_ms.map(|ms| (ms as i64).to_be_bytes().to_vec())),
        ));

        // idx_lease_id: Query keys by lease
        let lease_space = index_subspace.subspace(&Tuple::new().push("lease_id"));
        let _ = registry.register(SecondaryIndex::numeric(
            "idx_lease_id",
            lease_space,
            Arc::new(|entry| entry.lease_id.map(|id| (id as i64).to_be_bytes().to_vec())),
        ));

        registry
    }

    /// Register a new index.
    ///
    /// # Errors
    ///
    /// Returns `IndexError::TooManyIndexes` if the registry is at capacity.
    pub fn register(&mut self, index: SecondaryIndex) -> IndexResult<()> {
        if self.indexes.len() >= MAX_INDEXES as usize {
            return Err(IndexError::TooManyIndexes);
        }
        self.indexes.insert(index.name().to_owned(), index);
        Ok(())
    }

    /// Get an index by name.
    pub fn get(&self, name: &str) -> Option<&SecondaryIndex> {
        self.indexes.get(name)
    }

    /// Iterate over all indexes.
    pub fn iter(&self) -> impl Iterator<Item = &SecondaryIndex> {
        self.indexes.values()
    }

    /// Get the number of registered indexes.
    pub fn len(&self) -> usize {
        self.indexes.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.indexes.is_empty()
    }

    /// Get all index names.
    pub fn names(&self) -> Vec<&str> {
        self.indexes.keys().map(|s| s.as_str()).collect()
    }

    /// Get definitions for all registered indexes.
    ///
    /// This can be used to persist index metadata and recreate
    /// indexes on startup.
    pub fn definitions(&self) -> Vec<IndexDefinition> {
        // Return definitions for the builtin indexes
        // Custom indexes would need to be tracked separately
        IndexDefinition::builtins()
    }

    /// Remove an index by name.
    ///
    /// Note: Built-in indexes cannot be removed; this is a no-op for them.
    pub fn unregister(&mut self, name: &str) -> bool {
        // Check if it's a builtin (don't allow removal)
        let is_builtin = matches!(name, "idx_mod_revision" | "idx_create_revision" | "idx_expires_at" | "idx_lease_id");

        if is_builtin {
            return false;
        }

        self.indexes.remove(name).is_some()
    }

    /// Get a reference to the internal indexes map (for update generation).
    pub(super) fn indexes(&self) -> &BTreeMap<String, SecondaryIndex> {
        &self.indexes
    }
}

impl Default for IndexRegistry {
    fn default() -> Self {
        Self::new()
    }
}
