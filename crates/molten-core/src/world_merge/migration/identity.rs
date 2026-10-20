use std::collections::BTreeMap;

use kamacite_core::PreservesLimits;
use kamacite_core::PreservesSchema;
use kamacite_core::encode_typed_preserves;
use serde::Serialize;

use super::super::WorldMergeIssue;
use super::super::WorldMergeRequest;
use super::super::WorldMergeValue;
use super::AdmittedWorldMigration;

pub(in crate::world_merge) fn bytes_ref(bytes: &[u8]) -> String {
    format!("blake3:{}", blake3::hash(bytes).to_hex())
}

pub(in crate::world_merge) fn canonical_ref<T: Serialize>(
    value: &T,
    label: &'static str,
) -> Result<String, WorldMergeIssue> {
    let artifact = encode_typed_preserves(
        value,
        PreservesSchema {
            schema_version: label,
            root_label: label,
        },
        PreservesLimits {
            max_input_bytes: 8_388_608,
            max_output_bytes: 8_388_608,
            max_atom_bytes: 1_048_576,
            max_collection_items: 65_536,
            max_values: 131_072,
            max_depth: 64,
        },
    )
    .map_err(|_| WorldMergeIssue::InvalidBounds("migration_identity"))?;
    Ok(bytes_ref(&artifact.packed_bytes))
}

#[derive(Serialize)]
#[serde(rename = "world-merge-migration-payload-v1")]
struct PayloadIdentity<'a> {
    bytes: String,
    keys: Vec<(&'a str, String)>,
}

pub(in crate::world_merge) fn payload_ref(
    bytes: &[u8],
    keyed: &BTreeMap<String, Vec<u8>>,
) -> Result<String, WorldMergeIssue> {
    canonical_ref(
        &PayloadIdentity {
            bytes: bytes_ref(bytes),
            keys: keyed.iter().map(|(key, bytes)| (key.as_str(), bytes_ref(bytes))).collect(),
        },
        "world-merge-migration-payload-v1",
    )
}

#[derive(Serialize)]
#[serde(rename = "world-merge-original-value-v1")]
struct ValueIdentity<'a> {
    root: Option<&'a str>,
    schema: Option<&'a str>,
    available: bool,
    bytes: Option<String>,
    keys: Vec<(&'a str, String)>,
}

fn value_ref(value: &WorldMergeValue) -> Result<String, WorldMergeIssue> {
    canonical_ref(
        &ValueIdentity {
            root: value.root.as_ref().map(|root| root.as_str()),
            schema: value.schema_ref.as_ref().map(|schema| schema.as_str()),
            available: value.available,
            bytes: value.canonical_bytes.as_deref().map(bytes_ref),
            keys: value.keyed_values.iter().map(|(key, bytes)| (key.as_str(), bytes_ref(bytes))).collect(),
        },
        "world-merge-original-value-v1",
    )
}

type HandlerIdentity<'a> = (&'a str, &'a str, &'a str, &'a str, &'a str, &'a str, u64, bool);

#[derive(Serialize)]
#[serde(rename = "world-merge-original-request-v1")]
struct RequestIdentity<'a> {
    base: &'a str,
    sources: Vec<&'a str>,
    ancestor_verified: bool,
    ancestor_ambiguous: bool,
    profile: &'a str,
    policy: &'a str,
    bounds: (u32, u32, u64, u32),
    roots: Vec<(&'a str, String, String, String)>,
    modes: Vec<(&'a str, &'a str)>,
    migrations: Vec<(&'a str, &'a str, &'a str, &'a str, &'a str, bool)>,
    handlers: Vec<HandlerIdentity<'a>>,
}

pub(in crate::world_merge) fn request_ref(request: &WorldMergeRequest) -> Result<String, WorldMergeIssue> {
    let mut sources = request.source_heads.iter().map(|source| source.as_str()).collect::<Vec<_>>();
    sources.sort();
    let mut roots = request
        .roots
        .iter()
        .map(|root| Ok((root.kind.as_str(), value_ref(&root.base)?, value_ref(&root.left)?, value_ref(&root.right)?)))
        .collect::<Result<Vec<_>, WorldMergeIssue>>()?;
    roots.sort_by_key(|root| root.0);
    canonical_ref(
        &RequestIdentity {
            base: request.base_head.as_str(),
            sources,
            ancestor_verified: request.common_ancestor_verified,
            ancestor_ambiguous: request.common_ancestor_ambiguous,
            profile: request.profile.profile_ref.as_str(),
            policy: request.profile.policy_ref.as_str(),
            bounds: (
                request.bounds.max_roots,
                request.bounds.max_keys,
                request.bounds.max_value_bytes,
                request.bounds.max_conflicts,
            ),
            roots,
            modes: request.profile.root_modes.iter().map(|(kind, mode)| (kind.as_str(), mode.as_str())).collect(),
            migrations: request
                .profile
                .migrations
                .iter()
                .map(|(kind, binding)| {
                    (
                        kind.as_str(),
                        binding.plan_ref.as_str(),
                        binding.profile_id.as_str(),
                        binding.source_schema.as_str(),
                        binding.target_schema.as_str(),
                        binding.admitted,
                    )
                })
                .collect(),
            handlers: request
                .profile
                .handlers
                .iter()
                .map(|(kind, profile)| {
                    (
                        kind.as_str(),
                        profile.handler_ref.as_str(),
                        profile.behavior_ref.as_str(),
                        profile.input_schema.as_str(),
                        profile.output_schema.as_str(),
                        profile.policy_ref.as_str(),
                        profile.max_output_bytes,
                        profile.pure,
                    )
                })
                .collect(),
        },
        "world-merge-original-request-v1",
    )
}

#[derive(Serialize)]
#[serde(rename = "world-merge-admitted-conversion-v1")]
struct ConversionIdentity<'a> {
    request: &'a str,
    kind: &'a str,
    side: &'a str,
    original_root: &'a str,
    source_content: &'a str,
    source_schema: &'a str,
    target_schema: &'a str,
    plan: &'a str,
    profile: &'a str,
    max_output_bytes: u64,
    max_keys: u32,
}

pub(super) fn conversion_ref(context: &str, conversion: &AdmittedWorldMigration) -> Result<String, WorldMergeIssue> {
    canonical_ref(
        &ConversionIdentity {
            request: context,
            kind: conversion.kind.as_str(),
            side: conversion.side.as_str(),
            original_root: conversion.original_root.as_str(),
            source_content: &conversion.source_ref,
            source_schema: conversion.binding.source_schema.as_str(),
            target_schema: conversion.binding.target_schema.as_str(),
            plan: conversion.binding.plan_ref.as_str(),
            profile: &conversion.binding.profile_id,
            max_output_bytes: conversion.max_output_bytes,
            max_keys: conversion.max_keys,
        },
        "world-merge-admitted-conversion-v1",
    )
}
