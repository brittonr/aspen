use std::collections::BTreeMap;

use super::super::*;
use crate::world_commit::RootKind;
use crate::world_commit::WorldCommitRef;
use crate::world_commit::WorldRootRef;

pub(super) fn reference(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}

pub(super) fn commit(label: &str) -> WorldCommitRef {
    WorldCommitRef::new(reference(label)).expect("commit ref")
}

pub(super) fn root(kind: RootKind, label: &str) -> WorldRootRef {
    WorldRootRef::parse(kind, reference(label)).expect("root ref")
}

pub(super) fn schema(label: &str) -> WorldMergeSchemaRef {
    WorldMergeSchemaRef::new(reference(label)).expect("schema ref")
}

pub(super) fn value(kind: RootKind, label: &str) -> WorldMergeValue {
    WorldMergeValue {
        root: Some(root(kind, label)),
        schema_ref: Some(schema(&format!("{}-schema", kind.as_str()))),
        available: true,
        canonical_bytes: Some(label.as_bytes().to_vec()),
        keyed_values: BTreeMap::new(),
    }
}

pub(super) fn keyed(label: &str, entries: &[(&str, &str)]) -> WorldMergeValue {
    WorldMergeValue {
        root: Some(root(RootKind::DurableState, label)),
        schema_ref: Some(schema("durable-schema")),
        available: true,
        canonical_bytes: None,
        keyed_values: entries.iter().map(|(key, value)| ((*key).to_string(), value.as_bytes().to_vec())).collect(),
    }
}

pub(super) fn profile() -> WorldMergeProfile {
    WorldMergeProfile {
        profile_ref: WorldMergeProfileRef::new(reference("merge-profile")).expect("profile ref"),
        policy_ref: WorldMergePolicyRef::new(reference("merge-policy")).expect("policy ref"),
        root_modes: BTreeMap::from([
            (RootKind::Artifact, WorldMergeMode::IdenticalOnly),
            (RootKind::DurableState, WorldMergeMode::AncestorReplacement),
        ]),
        migrations: BTreeMap::new(),
        handlers: BTreeMap::new(),
    }
}

pub(super) fn request() -> WorldMergeRequest {
    WorldMergeRequest {
        base_head: commit("base"),
        source_heads: vec![commit("left"), commit("right")],
        common_ancestor_verified: true,
        common_ancestor_ambiguous: false,
        roots: vec![
            WorldMergeRootInput {
                kind: RootKind::Artifact,
                base: value(RootKind::Artifact, "artifact"),
                left: value(RootKind::Artifact, "artifact"),
                right: value(RootKind::Artifact, "artifact"),
            },
            WorldMergeRootInput {
                kind: RootKind::DurableState,
                base: value(RootKind::DurableState, "base-state"),
                left: value(RootKind::DurableState, "left-state"),
                right: value(RootKind::DurableState, "base-state"),
            },
        ],
        profile: profile(),
        bounds: WorldMergeBounds::standard(),
    }
}
