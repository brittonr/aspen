use std::collections::BTreeMap;

use super::WorldMergeIssue;
use super::WorldMergeRootInput;
use super::WorldMergeSchemaRef;
use super::WorldMergeValue;
use super::WorldMigrationBinding;
use crate::world_commit::RootKind;
use crate::world_commit::WorldRootRef;

mod admission;
mod identity;
mod preparation;
mod result;

pub use admission::admit_world_migrations;
pub(super) use identity::canonical_ref;
pub(super) use identity::payload_ref;
pub(super) use preparation::prepare_values;
pub(super) use preparation::restore_generated_outputs;
pub use result::validate_world_migration_result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldMergeInputSide {
    Base,
    Left,
    Right,
}

impl WorldMergeInputSide {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Base => "base",
            Self::Left => "left",
            Self::Right => "right",
        }
    }

    pub fn value(self, root: &WorldMergeRootInput) -> &WorldMergeValue {
        match self {
            Self::Base => &root.base,
            Self::Left => &root.left,
            Self::Right => &root.right,
        }
    }

    fn value_mut(self, root: &mut WorldMergeRootInput) -> &mut WorldMergeValue {
        match self {
            Self::Base => &mut root.base,
            Self::Left => &mut root.left,
            Self::Right => &mut root.right,
        }
    }
}

/// Constructed only by admission over the original, bounded request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedWorldMigration {
    kind: RootKind,
    side: WorldMergeInputSide,
    binding: WorldMigrationBinding,
    original_root: WorldRootRef,
    source_ref: String,
    conversion_ref: String,
    max_output_bytes: u64,
    max_keys: u32,
}

impl AdmittedWorldMigration {
    pub const fn kind(&self) -> RootKind {
        self.kind
    }
    pub const fn side(&self) -> WorldMergeInputSide {
        self.side
    }
    pub fn binding(&self) -> &WorldMigrationBinding {
        &self.binding
    }
    pub fn original_root(&self) -> &WorldRootRef {
        &self.original_root
    }
    pub fn source_ref(&self) -> &str {
        &self.source_ref
    }
    pub fn conversion_ref(&self) -> &str {
        &self.conversion_ref
    }
    pub const fn max_output_bytes(&self) -> u64 {
        self.max_output_bytes
    }
}

/// Adapter observation, not admission evidence. All fields are checked by the core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldMigrationResult {
    pub conversion_ref: String,
    pub target_schema: WorldMergeSchemaRef,
    pub result_ref: String,
    pub canonical_bytes: Vec<u8>,
    pub keyed_values: BTreeMap<String, Vec<u8>>,
}

impl WorldMigrationResult {
    pub fn new(
        conversion: &AdmittedWorldMigration,
        canonical_bytes: Vec<u8>,
        keyed_values: BTreeMap<String, Vec<u8>>,
    ) -> Result<Self, WorldMergeIssue> {
        result::validate_payload(conversion, &canonical_bytes, &keyed_values)?;
        let result_ref = identity::payload_ref(&canonical_bytes, &keyed_values)?;
        Ok(Self {
            conversion_ref: conversion.conversion_ref.clone(),
            target_schema: conversion.binding.target_schema.clone(),
            result_ref,
            canonical_bytes,
            keyed_values,
        })
    }
}

/// Validated bytes cannot be substituted independently of their conversion evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterializedWorldMigration {
    evidence: WorldMigrationEvidence,
    canonical_bytes: Vec<u8>,
    keyed_values: BTreeMap<String, Vec<u8>>,
}

impl MaterializedWorldMigration {
    pub fn conversion(&self) -> &AdmittedWorldMigration {
        &self.evidence.conversion
    }
    pub fn result_ref(&self) -> &str {
        &self.evidence.result_ref
    }
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }
    pub fn keyed_values(&self) -> &BTreeMap<String, Vec<u8>> {
        &self.keyed_values
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldMigrationEvidence {
    conversion: AdmittedWorldMigration,
    result_ref: String,
}

impl WorldMigrationEvidence {
    pub fn conversion(&self) -> &AdmittedWorldMigration {
        &self.conversion
    }
    pub fn result_ref(&self) -> &str {
        &self.result_ref
    }
}
