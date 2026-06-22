use std::collections::BTreeSet;
use std::path::Path;

use preserves::IOValue;
use preserves::Value;

use crate::chunk_store;
use crate::error::MoltenError;
use crate::error::Result;
use crate::evidence::SIGNATURE_ALGORITHM;
use crate::ledger;
use crate::preserves_rail::FEDERATION_ANNOUNCEMENT_SCHEMA;
use crate::preserves_rail::FEDERATION_INVENTORY_SCHEMA;
use crate::preserves_rail::FEDERATION_RECEIPT_SCHEMA;
use crate::preserves_rail::canonical_bytes;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::content_ref_from_bytes;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::validate_content_ref;
use crate::preserves_rail::value_to_iovalue;
use crate::runtime::RuntimeAssertion;
use crate::runtime::RuntimeValue;

pub const FEDERATION_ANNOUNCEMENT_PURPOSE: &str = "federation-announcement";
pub const FEDERATION_INVENTORY_PURPOSE: &str = "federation-inventory";
pub const RESOURCE_ARTIFACT: &str = "artifact";
pub const RESOURCE_CHUNK_MANIFEST: &str = "chunk-manifest";
pub const RESOURCE_CHUNK: &str = "chunk";
pub const RESOURCE_DOC_METADATA: &str = "doc-metadata";
pub const RESOURCE_CATALOG_METADATA: &str = "catalog-metadata";
pub const RESOURCE_RECEIPT: &str = "receipt";
pub const RESOURCE_PROVENANCE: &str = "provenance";
pub const RESOURCE_TRANSCRIPT: &str = "transcript";
pub const RESOURCE_PROTOCOL: &str = "protocol";
pub const RESOURCE_SCHEMA: &str = "schema";

const MAX_FEDERATION_RESOURCES: usize = 4096;
const MAX_FEDERATION_ASSERTIONS: usize = 8192;

const _: () = assert!(MAX_FEDERATION_RESOURCES <= 100_000);
const _: () = assert!(MAX_FEDERATION_ASSERTIONS <= 100_000);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FederatedResource {
    pub resource_type: String,
    pub resource_ref: String,
    pub schema: String,
    pub transport: String,
    pub source_peer: String,
}

impl FederatedResource {
    pub fn new(
        resource_type: impl Into<String>,
        resource_ref: impl Into<String>,
        schema: impl Into<String>,
        transport: impl Into<String>,
        source_peer: impl Into<String>,
    ) -> Self {
        Self {
            resource_type: resource_type.into(),
            resource_ref: resource_ref.into(),
            schema: schema.into(),
            transport: transport.into(),
            source_peer: source_peer.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FederationAnnouncement {
    pub announcement_ref: String,
    pub peer: String,
    pub resource: FederatedResource,
    pub signer: String,
    pub trust_root: String,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FederationInventory {
    pub inventory_ref: String,
    pub peer: String,
    pub resources: Vec<FederatedResource>,
    pub delegates: Vec<FederationDelegate>,
    pub signer: String,
    pub trust_root: String,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FederationDelegate {
    pub delegate_ref: String,
    pub resource_ref: String,
    pub capability: String,
    pub signer: String,
    pub trust_root: String,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FederationPullPolicy {
    pub allowed_resource_types: Vec<String>,
    pub required_delegate_capability: Option<String>,
    pub delegate_trust_root: String,
    pub delegate_key: String,
    pub max_resources: usize,
    pub max_imports: usize,
}

impl FederationPullPolicy {
    pub fn allow_all() -> Self {
        Self {
            allowed_resource_types: Vec::new(),
            required_delegate_capability: None,
            delegate_trust_root: String::new(),
            delegate_key: String::new(),
            max_resources: usize::MAX,
            max_imports: usize::MAX,
        }
    }

    pub fn allowed_types(allowed_resource_types: Vec<String>) -> Self {
        Self {
            allowed_resource_types,
            ..Self::allow_all()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FederationPull {
    pub peer: String,
    pub imported_refs: Vec<String>,
    pub skipped_refs: Vec<String>,
    pub denied_refs: Vec<String>,
    pub receipt_value: IOValue,
}

#[derive(Debug, Clone, Copy)]
pub struct AnnounceResourceInput<'a> {
    pub peer: &'a str,
    pub resource: &'a FederatedResource,
    pub signer: &'a str,
    pub trust_root: &'a str,
    pub key: &'a str,
    pub policy_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct InventoryWithDelegatesInput<'a> {
    pub peer: &'a str,
    pub resources: &'a [FederatedResource],
    pub delegates: &'a [FederationDelegate],
    pub signer: &'a str,
    pub trust_root: &'a str,
    pub key: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct PullLedgerInventoryInput<'a> {
    pub source_root: &'a Path,
    pub dest_root: &'a Path,
    pub inventory_value: &'a IOValue,
    pub trust_root: &'a str,
    pub key: &'a str,
    pub allowed_resource_types: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct PullLedgerInventoryPolicyInput<'a> {
    pub source_root: &'a Path,
    pub dest_root: &'a Path,
    pub inventory_value: &'a IOValue,
    pub trust_root: &'a str,
    pub key: &'a str,
    pub policy: &'a FederationPullPolicy,
}

#[derive(Debug, Clone, Copy)]
pub struct PullChunkManifestInput<'a> {
    pub iroh_root: &'a Path,
    pub dest_root: &'a Path,
    pub announcement_value: &'a IOValue,
    pub trust_root: &'a str,
    pub key: &'a str,
    pub peer: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct FederationReceiptValueInput<'a> {
    pub operation: &'a str,
    pub decision: &'a str,
    pub peer: &'a str,
    pub resources: &'a [FederatedResource],
    pub imported_refs: &'a [String],
    pub skipped_refs: &'a [String],
    pub denied_refs: &'a [String],
}

pub fn announce_resource(input: &AnnounceResourceInput<'_>) -> Result<FederationAnnouncement> {
    validate_peer(input.peer)?;
    validate_resource(input.resource)?;
    validate_refs(input.policy_refs, "federation announcement policy ref")?;
    let payload = announcement_payload_value(input.peer, input.resource, input.policy_refs);
    let signature =
        signature_record(&payload, input.signer, FEDERATION_ANNOUNCEMENT_PURPOSE, input.trust_root, input.key)?;
    let value = record("federation-announcement-v1", vec![
        string(FEDERATION_ANNOUNCEMENT_SCHEMA),
        record("payload", vec![payload]),
        signature,
        record("checks", vec![sequence(vec![
            record("check", vec![string("signed-announcement"), string("pass")]),
            record("check", vec![string("resource-ref-binding"), string("pass")]),
            record("check", vec![string("announcement-is-a-hint"), string("pass")]),
        ])]),
    ]);
    parse_announcement(&value, input.trust_root, input.key)
}

pub fn inventory_ledger(
    root: &Path,
    peer: &str,
    signer: &str,
    trust_root: &str,
    key: &str,
) -> Result<FederationInventory> {
    validate_peer(peer)?;
    let resources = ledger::list_artifacts(root)?
        .into_iter()
        .map(|entry| {
            FederatedResource::new(
                entry.artifact_kind,
                entry.artifact_ref,
                "molten.ledger.artifact.v1",
                "ledger-local",
                peer,
            )
        })
        .collect::<Vec<_>>();
    inventory_for_resources(peer, &resources, signer, trust_root, key)
}

pub fn inventory_for_resources(
    peer: &str,
    resources: &[FederatedResource],
    signer: &str,
    trust_root: &str,
    key: &str,
) -> Result<FederationInventory> {
    inventory_for_resources_with_delegates(&InventoryWithDelegatesInput {
        peer,
        resources,
        delegates: &[],
        signer,
        trust_root,
        key,
    })
}

pub fn inventory_for_resources_with_delegates(input: &InventoryWithDelegatesInput<'_>) -> Result<FederationInventory> {
    validate_peer(input.peer)?;
    for resource in input.resources {
        validate_resource(resource)?;
    }
    for delegate in input.delegates {
        require_ref(&delegate.resource_ref, "federation delegate resource ref")?;
    }
    let payload = inventory_payload_value(input.peer, input.resources, input.delegates);
    let signature =
        signature_record(&payload, input.signer, FEDERATION_INVENTORY_PURPOSE, input.trust_root, input.key)?;
    let value = record("federation-inventory-v1", vec![
        string(FEDERATION_INVENTORY_SCHEMA),
        record("payload", vec![payload]),
        signature,
        record("checks", vec![sequence(vec![
            record("check", vec![string("signed-inventory"), string("pass")]),
            record("check", vec![string("static-peer-discovery"), string("pass")]),
            record("check", vec![string("inventory-is-a-hint"), string("pass")]),
            record("check", vec![string("no-global-consistency-claim"), string("pass")]),
        ])]),
    ]);
    parse_inventory(&value, input.trust_root, input.key)
}

pub fn delegate_resource(
    resource: &FederatedResource,
    capability: &str,
    signer: &str,
    trust_root: &str,
    key: &str,
) -> Result<FederationDelegate> {
    validate_resource(resource)?;
    if capability.trim().is_empty() {
        return Err(MoltenError::invalid_harness("federation delegate capability must not be empty"));
    }
    let payload = record("federation-delegate-payload", vec![
        resource_value(resource),
        record("capability", vec![string(capability)]),
    ]);
    let signature = signature_record(&payload, signer, "federation-delegate", trust_root, key)?;
    let value = record("federation-delegate-v1", vec![payload, signature]);
    parse_delegate(&value, Some(&resource.resource_ref), Some(capability), trust_root, key)
}

pub fn parse_delegate(
    value: &IOValue,
    expected_resource_ref: Option<&str>,
    expected_capability: Option<&str>,
    trust_root: &str,
    key: &str,
) -> Result<FederationDelegate> {
    let fields = value
        .collect_simple_record("federation-delegate-v1", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("expected <federation-delegate-v1 ...>"))?;
    let payload = value_to_iovalue(&fields[0]);
    let payload_fields = payload
        .collect_simple_record("federation-delegate-payload", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("expected federation delegate payload"))?;
    let resource = parse_resource(&value_to_iovalue(&payload_fields[0]))?;
    let capability = record_string(&payload_fields[1], "capability")?;
    if let Some(expected_resource_ref) = expected_resource_ref
        && resource.resource_ref != expected_resource_ref
    {
        return Err(MoltenError::invalid_harness("federation delegate resource binding mismatch"));
    }
    if let Some(expected_capability) = expected_capability
        && capability != expected_capability
    {
        return Err(MoltenError::invalid_harness("federation delegate capability mismatch"));
    }
    let (signer, actual_trust_root) =
        verify_signature_record(&fields[1], &payload, "federation-delegate", trust_root, key)?;
    Ok(FederationDelegate {
        delegate_ref: canonical_hash(value)?,
        resource_ref: resource.resource_ref,
        capability,
        signer,
        trust_root: actual_trust_root,
        value: value.clone(),
    })
}

fn parse_delegate_unverified(value: &IOValue) -> Result<FederationDelegate> {
    let fields = value
        .collect_simple_record("federation-delegate-v1", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("expected <federation-delegate-v1 ...>"))?;
    let payload = value_to_iovalue(&fields[0]);
    let payload_fields = payload
        .collect_simple_record("federation-delegate-payload", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("expected federation delegate payload"))?;
    let resource = parse_resource(&value_to_iovalue(&payload_fields[0]))?;
    let capability = record_string(&payload_fields[1], "capability")?;
    let signature = value_to_iovalue(&fields[1]);
    let signature_fields = signature
        .collect_simple_record("signature", Some(5))
        .ok_or_else(|| MoltenError::invalid_harness("expected federation signature"))?;
    Ok(FederationDelegate {
        delegate_ref: canonical_hash(value)?,
        resource_ref: resource.resource_ref,
        capability,
        signer: record_string(&signature_fields[0], "signer")?,
        trust_root: record_string(&signature_fields[2], "trust-root")?,
        value: value.clone(),
    })
}

pub fn parse_announcement(value: &IOValue, trust_root: &str, key: &str) -> Result<FederationAnnouncement> {
    let fields = value
        .collect_simple_record("federation-announcement-v1", Some(4))
        .ok_or_else(|| MoltenError::invalid_harness("expected <federation-announcement-v1 ...>"))?;
    require_schema(&fields[0], FEDERATION_ANNOUNCEMENT_SCHEMA, "federation announcement schema")?;
    let payload = record_value(&fields[1], "payload")?;
    let (peer, resource, _policy_refs) = parse_announcement_payload(&payload)?;
    let (signer, actual_trust_root) =
        verify_signature_record(&fields[2], &payload, FEDERATION_ANNOUNCEMENT_PURPOSE, trust_root, key)?;
    let checks = parse_checks(&fields[3])?;
    require_check(&checks, "signed-announcement")?;
    require_check(&checks, "announcement-is-a-hint")?;
    Ok(FederationAnnouncement {
        announcement_ref: canonical_hash(value)?,
        peer,
        resource,
        signer,
        trust_root: actual_trust_root,
        value: value.clone(),
    })
}

pub fn parse_inventory(value: &IOValue, trust_root: &str, key: &str) -> Result<FederationInventory> {
    let fields = value
        .collect_simple_record("federation-inventory-v1", Some(4))
        .ok_or_else(|| MoltenError::invalid_harness("expected <federation-inventory-v1 ...>"))?;
    require_schema(&fields[0], FEDERATION_INVENTORY_SCHEMA, "federation inventory schema")?;
    let payload = record_value(&fields[1], "payload")?;
    let (peer, resources, delegates) = parse_inventory_payload(&payload)?;
    let (signer, actual_trust_root) =
        verify_signature_record(&fields[2], &payload, FEDERATION_INVENTORY_PURPOSE, trust_root, key)?;
    let checks = parse_checks(&fields[3])?;
    require_check(&checks, "signed-inventory")?;
    require_check(&checks, "no-global-consistency-claim")?;
    Ok(FederationInventory {
        inventory_ref: canonical_hash(value)?,
        peer,
        resources,
        delegates,
        signer,
        trust_root: actual_trust_root,
        value: value.clone(),
    })
}

pub fn pull_ledger_inventory(input: &PullLedgerInventoryInput<'_>) -> Result<FederationPull> {
    pull_ledger_inventory_with_policy(&PullLedgerInventoryPolicyInput {
        source_root: input.source_root,
        dest_root: input.dest_root,
        inventory_value: input.inventory_value,
        trust_root: input.trust_root,
        key: input.key,
        policy: &FederationPullPolicy::allowed_types(input.allowed_resource_types.to_vec()),
    })
}

pub fn pull_ledger_inventory_with_policy(input: &PullLedgerInventoryPolicyInput<'_>) -> Result<FederationPull> {
    let inventory = parse_inventory(input.inventory_value, input.trust_root, input.key)?;
    if inventory.resources.len() > input.policy.max_resources {
        return Ok(oversized_pull(&inventory));
    }
    let refs = PullEnv::new(input, &inventory)?.collect_refs()?;
    Ok(finish_pull(inventory, refs))
}

#[derive(Default)]
struct PullRefs {
    imported_refs: Vec<String>,
    skipped_refs: Vec<String>,
    denied_refs: Vec<String>,
}

impl PullRefs {
    fn deny(&mut self, resource: &FederatedResource) -> Result<()> {
        push_bounded(
            &mut self.denied_refs,
            resource.resource_ref.clone(),
            MAX_FEDERATION_RESOURCES,
            "federation denied refs",
        )
    }

    fn skip(&mut self, resource: &FederatedResource) -> Result<()> {
        push_bounded(
            &mut self.skipped_refs,
            resource.resource_ref.clone(),
            MAX_FEDERATION_RESOURCES,
            "federation skipped refs",
        )
    }

    fn import(&mut self, resource: &FederatedResource) -> Result<()> {
        push_bounded(
            &mut self.imported_refs,
            resource.resource_ref.clone(),
            MAX_FEDERATION_RESOURCES,
            "federation imported refs",
        )
    }
}

struct PullEnv<'a, 'b> {
    input: &'a PullLedgerInventoryPolicyInput<'b>,
    inventory: &'a FederationInventory,
    existing: BTreeSet<String>,
    allowed: BTreeSet<String>,
}

impl<'a, 'b> PullEnv<'a, 'b> {
    fn new(input: &'a PullLedgerInventoryPolicyInput<'b>, inventory: &'a FederationInventory) -> Result<Self> {
        let existing = ledger::list_artifacts(input.dest_root)?
            .into_iter()
            .map(|entry| entry.artifact_ref)
            .collect::<BTreeSet<_>>();
        ensure_count_at_most(inventory.resources.len(), MAX_FEDERATION_RESOURCES, "federation inventory resources")?;
        let allowed = input.policy.allowed_resource_types.iter().cloned().collect::<BTreeSet<_>>();
        Ok(Self {
            input,
            inventory,
            existing,
            allowed,
        })
    }

    fn collect_refs(&self) -> Result<PullRefs> {
        let mut refs = PullRefs::default();
        for resource in &self.inventory.resources {
            self.apply_resource(&mut refs, resource)?;
        }
        Ok(refs)
    }

    fn apply_resource(&self, refs: &mut PullRefs, resource: &FederatedResource) -> Result<()> {
        if self.is_type_denied(resource) || self.is_delegate_missing(resource)? {
            return refs.deny(resource);
        }
        if self.is_duplicate(refs, resource) {
            return refs.skip(resource);
        }
        if refs.imported_refs.len() >= self.input.policy.max_imports {
            return refs.deny(resource);
        }
        self.import_verified(refs, resource)
    }

    fn is_type_denied(&self, resource: &FederatedResource) -> bool {
        !self.allowed.is_empty() && !self.allowed.contains(&resource.resource_type)
    }

    fn is_delegate_missing(&self, resource: &FederatedResource) -> Result<bool> {
        if let Some(capability) = self.input.policy.required_delegate_capability.as_deref() {
            return Ok(!has_valid_delegate(
                &self.inventory.delegates,
                resource,
                capability,
                &self.input.policy.delegate_trust_root,
                &self.input.policy.delegate_key,
            )?);
        }
        Ok(false)
    }

    fn is_duplicate(&self, refs: &PullRefs, resource: &FederatedResource) -> bool {
        self.existing.contains(&resource.resource_ref)
            || refs.imported_refs.iter().any(|imported| imported == &resource.resource_ref)
    }

    fn import_verified(&self, refs: &mut PullRefs, resource: &FederatedResource) -> Result<()> {
        let artifact = match ledger::read_artifact(self.input.source_root, &resource.resource_ref) {
            Ok(artifact) => artifact,
            Err(_) => return refs.deny(resource),
        };
        let actual_ref = canonical_hash(&artifact)?;
        if actual_ref != resource.resource_ref || ledger::artifact_kind(&artifact) != resource.resource_type {
            return refs.deny(resource);
        }
        ledger::import_artifact(self.input.dest_root, &artifact)?;
        refs.import(resource)
    }
}

fn oversized_pull(inventory: &FederationInventory) -> FederationPull {
    let denied_refs = inventory.resources.iter().map(|resource| resource.resource_ref.clone()).collect::<Vec<_>>();
    FederationPull {
        peer: inventory.peer.clone(),
        imported_refs: Vec::new(),
        skipped_refs: Vec::new(),
        denied_refs: denied_refs.clone(),
        receipt_value: federation_receipt_value(&FederationReceiptValueInput {
            operation: "pull-ledger-inventory",
            decision: "fail",
            peer: &inventory.peer,
            resources: &inventory.resources,
            imported_refs: &[],
            skipped_refs: &[],
            denied_refs: &denied_refs,
        }),
    }
}

fn finish_pull(inventory: FederationInventory, refs: PullRefs) -> FederationPull {
    let decision = if refs.denied_refs.is_empty() { "pass" } else { "fail" };
    let receipt_value = federation_receipt_value(&FederationReceiptValueInput {
        operation: "pull-ledger-inventory",
        decision,
        peer: &inventory.peer,
        resources: &inventory.resources,
        imported_refs: &refs.imported_refs,
        skipped_refs: &refs.skipped_refs,
        denied_refs: &refs.denied_refs,
    });
    FederationPull {
        peer: inventory.peer,
        imported_refs: refs.imported_refs,
        skipped_refs: refs.skipped_refs,
        denied_refs: refs.denied_refs,
        receipt_value,
    }
}

pub fn pull_chunk_manifest_from_announcement(input: &PullChunkManifestInput<'_>) -> Result<FederationPull> {
    let announcement = parse_announcement(input.announcement_value, input.trust_root, input.key)?;
    if announcement.resource.resource_type != RESOURCE_CHUNK_MANIFEST {
        return Err(MoltenError::invalid_harness("federation chunk pull requires a chunk-manifest resource"));
    }
    let fetched = chunk_store::fetch_iroh_blobs(
        input.iroh_root,
        input.dest_root,
        &announcement.resource.transport,
        Some(&announcement.resource.resource_ref),
        input.peer,
    )?;
    let imported_refs = std::iter::once(fetched.manifest_ref.clone())
        .chain(fetched.fetched_chunks.iter().cloned())
        .collect::<Vec<_>>();
    let resources = vec![announcement.resource.clone()];
    let receipt_value = federation_receipt_value(&FederationReceiptValueInput {
        operation: "pull-chunk-manifest",
        decision: "pass",
        peer: &announcement.peer,
        resources: &resources,
        imported_refs: &imported_refs,
        skipped_refs: &[],
        denied_refs: &[],
    });
    Ok(FederationPull {
        peer: announcement.peer,
        imported_refs,
        skipped_refs: Vec::new(),
        denied_refs: Vec::new(),
        receipt_value,
    })
}

pub fn federation_status_assertions(pull: &FederationPull) -> Result<Vec<RuntimeAssertion>> {
    let mut assertions = Vec::new();
    push_bounded(
        &mut assertions,
        RuntimeAssertion {
            actor: "federation".to_string(),
            value: RuntimeValue::new(record("federation-sync-status", vec![
                record("peer", vec![string(&pull.peer)]),
                record("imported", vec![sequence(pull.imported_refs.iter().map(string).collect())]),
                record("skipped", vec![sequence(pull.skipped_refs.iter().map(string).collect())]),
                record("denied", vec![sequence(pull.denied_refs.iter().map(string).collect())]),
            ]))?,
        },
        MAX_FEDERATION_ASSERTIONS,
        "federation status assertions",
    )?;
    for imported in &pull.imported_refs {
        push_bounded(
            &mut assertions,
            RuntimeAssertion {
                actor: "federation".to_string(),
                value: RuntimeValue::new(record("federation-imported-resource", vec![
                    record("peer", vec![string(&pull.peer)]),
                    record("ref", vec![string(imported)]),
                ]))?,
            },
            MAX_FEDERATION_ASSERTIONS,
            "federation status assertions",
        )?;
    }
    for denied in &pull.denied_refs {
        push_bounded(
            &mut assertions,
            RuntimeAssertion {
                actor: "federation".to_string(),
                value: RuntimeValue::new(record("federation-denied-resource", vec![
                    record("peer", vec![string(&pull.peer)]),
                    record("ref", vec![string(denied)]),
                ]))?,
            },
            MAX_FEDERATION_ASSERTIONS,
            "federation status assertions",
        )?;
    }
    Ok(assertions)
}

fn has_valid_delegate(
    delegates: &[FederationDelegate],
    resource: &FederatedResource,
    capability: &str,
    trust_root: &str,
    key: &str,
) -> Result<bool> {
    for delegate in delegates {
        if delegate.resource_ref == resource.resource_ref && delegate.capability == capability {
            parse_delegate(&delegate.value, Some(&resource.resource_ref), Some(capability), trust_root, key)?;
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn federation_receipt_value(input: &FederationReceiptValueInput<'_>) -> IOValue {
    record("federation-receipt-v1", vec![
        string(FEDERATION_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("peer", vec![string(input.peer)]),
        record("resources", vec![sequence(input.resources.iter().map(resource_value).collect())]),
        record("imports", vec![sequence(input.imported_refs.iter().map(string).collect())]),
        record("skipped", vec![sequence(input.skipped_refs.iter().map(string).collect())]),
        record("denied", vec![sequence(input.denied_refs.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("receiver-driven-pull"), string("pass")]),
            record("check", vec![string("signature-verified-before-fetch"), string("pass")]),
            record("check", vec![string("content-hash-verified-before-import"), string("pass")]),
            record("check", vec![
                string("local-policy-admission"),
                string(if input.denied_refs.is_empty() { "pass" } else { "fail" }),
            ]),
            record("check", vec![string("no-push-import"), string("pass")]),
        ])]),
    ])
}

fn announcement_payload_value(peer: &str, resource: &FederatedResource, policy_refs: &[String]) -> IOValue {
    record("federation-announcement-payload", vec![
        record("peer", vec![string(peer)]),
        resource_value(resource),
        record("policy", vec![sequence(policy_refs.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("resource-announced-by-origin"), string("pass")]),
            record("check", vec![string("pull-required-for-import"), string("pass")]),
        ])]),
    ])
}

fn inventory_payload_value(peer: &str, resources: &[FederatedResource], delegates: &[FederationDelegate]) -> IOValue {
    record("federation-inventory-payload", vec![
        record("peer", vec![string(peer)]),
        record("resources", vec![sequence(resources.iter().map(resource_value).collect())]),
        record("delegates", vec![sequence(
            delegates.iter().map(|delegate| delegate.value.clone()).collect(),
        )]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("receiver-computes-missing-set"), string("pass")]),
            record("check", vec![string("inventory-does-not-import"), string("pass")]),
            record("check", vec![string("delegate-capability-optional"), string("pass")]),
        ])]),
    ])
}

fn resource_value(resource: &FederatedResource) -> IOValue {
    record("federated-resource", vec![
        record("type", vec![string(&resource.resource_type)]),
        record("ref", vec![string(&resource.resource_ref)]),
        record("schema", vec![string(&resource.schema)]),
        record("transport", vec![string(&resource.transport)]),
        record("source-peer", vec![string(&resource.source_peer)]),
    ])
}

fn parse_announcement_payload(value: &IOValue) -> Result<(String, FederatedResource, Vec<String>)> {
    let fields = value
        .collect_simple_record("federation-announcement-payload", Some(4))
        .ok_or_else(|| MoltenError::invalid_harness("expected federation announcement payload"))?;
    let peer = record_string(&fields[0], "peer")?;
    let resource = parse_resource(&value_to_iovalue(&fields[1]))?;
    let policy_refs = parse_ref_sequence(&fields[2], "policy")?;
    let checks = parse_checks(&fields[3])?;
    require_check(&checks, "pull-required-for-import")?;
    Ok((peer, resource, policy_refs))
}

fn parse_inventory_payload(value: &IOValue) -> Result<(String, Vec<FederatedResource>, Vec<FederationDelegate>)> {
    let fields = value
        .collect_simple_record("federation-inventory-payload", Some(4))
        .ok_or_else(|| MoltenError::invalid_harness("expected federation inventory payload"))?;
    let peer = record_string(&fields[0], "peer")?;
    let resources_field = value_to_iovalue(&fields[1]);
    let resources_record = resources_field
        .collect_simple_record("resources", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected federation inventory resources"))?;
    let resource_values = resources_record[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("federation inventory resources must be a sequence"))?;
    let resources = resource_values
        .iter()
        .map(|resource| parse_resource(&value_to_iovalue(resource)))
        .collect::<Result<Vec<_>>>()?;
    let delegates_field = value_to_iovalue(&fields[2]);
    let delegates_record = delegates_field
        .collect_simple_record("delegates", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected federation inventory delegates"))?;
    let delegate_values = delegates_record[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("federation inventory delegates must be a sequence"))?;
    let delegates = delegate_values
        .iter()
        .map(|delegate| parse_delegate_unverified(&value_to_iovalue(delegate)))
        .collect::<Result<Vec<_>>>()?;
    let checks = parse_checks(&fields[3])?;
    require_check(&checks, "inventory-does-not-import")?;
    Ok((peer, resources, delegates))
}

fn parse_resource(value: &IOValue) -> Result<FederatedResource> {
    let fields = value
        .collect_simple_record("federated-resource", Some(5))
        .ok_or_else(|| MoltenError::invalid_harness("expected federated resource"))?;
    let resource = FederatedResource::new(
        record_string(&fields[0], "type")?,
        record_string(&fields[1], "ref")?,
        record_string(&fields[2], "schema")?,
        record_string(&fields[3], "transport")?,
        record_string(&fields[4], "source-peer")?,
    );
    validate_resource(&resource)?;
    Ok(resource)
}

fn signature_record(payload: &IOValue, signer: &str, purpose: &str, trust_root: &str, key: &str) -> Result<IOValue> {
    if signer.trim().is_empty() {
        return Err(MoltenError::invalid_harness("federation signer must not be empty"));
    }
    if trust_root.trim().is_empty() {
        return Err(MoltenError::invalid_harness("federation trust root must not be empty"));
    }
    Ok(record("signature", vec![
        record("signer", vec![string(signer)]),
        record("purpose", vec![string(purpose)]),
        record("trust-root", vec![string(trust_root)]),
        record("algorithm", vec![string(SIGNATURE_ALGORITHM)]),
        record("value", vec![string(&signature_for(payload, signer, purpose, trust_root, key)?)]),
    ]))
}

fn verify_signature_record(
    value: &Value<IOValue>,
    payload: &IOValue,
    purpose: &str,
    trust_root: &str,
    key: &str,
) -> Result<(String, String)> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record("signature", Some(5))
        .ok_or_else(|| MoltenError::invalid_harness("expected federation signature"))?;
    let signer = record_string(&fields[0], "signer")?;
    let actual_purpose = record_string(&fields[1], "purpose")?;
    if actual_purpose != purpose {
        return Err(MoltenError::invalid_harness(format!(
            "federation signature purpose {actual_purpose} does not match {purpose}"
        )));
    }
    let actual_trust_root = record_string(&fields[2], "trust-root")?;
    if actual_trust_root != trust_root {
        return Err(MoltenError::invalid_harness(format!(
            "federation signature trust root {actual_trust_root} does not match {trust_root}"
        )));
    }
    let algorithm = record_string(&fields[3], "algorithm")?;
    if algorithm != SIGNATURE_ALGORITHM {
        return Err(MoltenError::invalid_harness(format!("unsupported federation signature algorithm {algorithm}")));
    }
    let signature = record_string(&fields[4], "value")?;
    let expected = signature_for(payload, &signer, purpose, &actual_trust_root, key)?;
    if signature != expected {
        return Err(MoltenError::invalid_harness("federation signature verification failed"));
    }
    Ok((signer, actual_trust_root))
}

fn signature_for(payload: &IOValue, signer: &str, purpose: &str, trust_root: &str, key: &str) -> Result<String> {
    let mut material = canonical_bytes(payload)?;
    material.extend_from_slice(signer.as_bytes());
    material.push(0);
    material.extend_from_slice(purpose.as_bytes());
    material.push(0);
    material.extend_from_slice(trust_root.as_bytes());
    material.push(0);
    material.extend_from_slice(key.as_bytes());
    Ok(content_ref_from_bytes(&material))
}

fn validate_resource(resource: &FederatedResource) -> Result<()> {
    if resource.resource_type.trim().is_empty()
        || resource.schema.trim().is_empty()
        || resource.transport.trim().is_empty()
        || resource.source_peer.trim().is_empty()
    {
        return Err(MoltenError::invalid_harness("federated resource fields must not be empty"));
    }
    require_ref(&resource.resource_ref, "federated resource ref")
}

fn validate_peer(peer: &str) -> Result<()> {
    if peer.trim().is_empty() {
        Err(MoltenError::invalid_harness("federation peer must not be empty"))
    } else {
        Ok(())
    }
}

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    for reference in refs {
        require_ref(reference, field)?;
    }
    Ok(())
}

fn require_ref(reference: &str, field: &str) -> Result<()> {
    validate_content_ref(reference).map_err(|error| {
        MoltenError::invalid_harness(format!("expected canonical content ref for {field}, got {reference}: {error}"))
    })
}

fn parse_ref_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    let values = record[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {label}")))?;
    values
        .iter()
        .map(|value| {
            let reference = required_string(value, label)?;
            require_ref(&reference, label)?;
            Ok(reference)
        })
        .collect()
}

fn parse_checks(value: &Value<IOValue>) -> Result<Vec<(String, String)>> {
    let value = value_to_iovalue(value);
    let record = value
        .collect_simple_record("checks", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected federation checks"))?;
    let values = record[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("federation checks must be a sequence"))?;
    values
        .iter()
        .map(|check| {
            let check = value_to_iovalue(check);
            let fields = check
                .collect_simple_record("check", Some(2))
                .ok_or_else(|| MoltenError::invalid_harness("expected federation check"))?;
            Ok((required_string(&fields[0], "check name")?, required_string(&fields[1], "check status")?))
        })
        .collect()
}

fn require_check(checks: &[(String, String)], name: &str) -> Result<()> {
    if checks.iter().any(|(check, status)| check == name && status == "pass") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("federation evidence missing passing {name} check")))
    }
}

fn record_value(value: &Value<IOValue>, label: &str) -> Result<IOValue> {
    let value = value_to_iovalue(value);
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    Ok(value_to_iovalue(&record[0]))
}

fn record_string(value: &Value<IOValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    required_string(&record[0], label)
}

fn require_schema(value: &Value<IOValue>, expected: &str, field: &str) -> Result<()> {
    let actual = required_string(value, field)?;
    if actual != expected {
        return Err(MoltenError::invalid_harness(format!("expected {field} {expected}, got {actual}")));
    }
    Ok(())
}

fn required_string(value: &Value<IOValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn ensure_count_at_most(count: usize, maximum: usize, label: &str) -> Result<()> {
    if count > maximum {
        Err(MoltenError::invalid_harness(format!("{label} count {count} exceeds maximum {maximum}")))
    } else {
        Ok(())
    }
}

fn push_bounded<T>(values: &mut impl crate::bounded::VecSink<T>, value: T, maximum: usize, label: &str) -> Result<()> {
    let count = values
        .item_count()
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness(format!("{label} count overflow")))?;
    ensure_count_at_most(count, maximum, label)?;
    values.push_item(value);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::AtomicU64;
    use std::sync::atomic::Ordering;

    use hegel::TestCase;
    use hegel::generators;

    use super::*;

    #[test]
    fn signed_announcement_binds_resource_and_rejects_wrong_key() {
        let resource = FederatedResource::new(
            RESOURCE_ARTIFACT,
            ref_for("artifact"),
            "molten.ledger.artifact.v1",
            "ledger-local",
            "peer:source",
        );
        let announcement = announce_resource(&AnnounceResourceInput {
            peer: "peer:source",
            resource: &resource,
            signer: "peer:source",
            trust_root: "root",
            key: "key",
            policy_refs: &[],
        })
        .expect("announce");
        assert_eq!(announcement.resource, resource);
        let error = parse_announcement(&announcement.value, "root", "wrong-key").expect_err("wrong key rejected");
        assert!(error.to_string().contains("signature verification failed"));
    }

    #[test]
    fn signed_inventory_pull_imports_missing_ledger_artifacts_after_verification() {
        let source = temp_dir("federation-source");
        let destination = temp_dir("federation-destination");
        let artifact = record("federation-test-artifact", vec![string("hello")]);
        let imported = ledger::import_artifact(&source, &artifact).expect("source import");
        let inventory = inventory_ledger(&source, "peer:source", "peer:source", "root", "key").expect("inventory");
        assert_eq!(inventory.resources.len(), 1);
        let pull = pull_ledger_inventory(&PullLedgerInventoryInput {
            source_root: &source,
            dest_root: &destination,
            inventory_value: &inventory.value,
            trust_root: "root",
            key: "key",
            allowed_resource_types: &[],
        })
        .expect("pull");
        assert_eq!(pull.imported_refs, vec![imported.artifact_ref.clone()]);
        assert_eq!(ledger::read_artifact(&destination, &imported.artifact_ref).expect("read pulled"), artifact);
        assert!(ledger::artifact_kind(&pull.receipt_value) == "federation-receipt");
    }

    #[test]
    fn tampered_inventory_signature_rejects_before_import() {
        let source = temp_dir("federation-tamper-source");
        let destination = temp_dir("federation-tamper-destination");
        let artifact = record("federation-test-artifact", vec![string("hello")]);
        let imported = ledger::import_artifact(&source, &artifact).expect("source import");
        let inventory = inventory_ledger(&source, "peer:source", "peer:source", "root", "key").expect("inventory");
        let tampered_resource = FederatedResource::new(
            "artifact",
            imported.artifact_ref,
            "molten.ledger.artifact.v1",
            "ledger-tampered",
            "peer:source",
        );
        let tampered_payload = inventory_payload_value("peer:source", &[tampered_resource], &[]);
        let fields =
            inventory.value.collect_simple_record("federation-inventory-v1", Some(4)).expect("inventory fields");
        let tampered = record("federation-inventory-v1", vec![
            value_to_iovalue(&fields[0]),
            record("payload", vec![tampered_payload]),
            value_to_iovalue(&fields[2]),
            value_to_iovalue(&fields[3]),
        ]);
        let error = pull_ledger_inventory(&PullLedgerInventoryInput {
            source_root: &source,
            dest_root: &destination,
            inventory_value: &tampered,
            trust_root: "root",
            key: "key",
            allowed_resource_types: &[],
        })
        .expect_err("tampered inventory rejected");
        assert!(error.to_string().contains("signature verification failed"));
        assert!(ledger::list_artifacts(&destination).expect("destination list").is_empty());
    }

    #[test]
    fn delegate_capability_is_required_when_policy_demands_it() {
        let source = temp_dir("federation-delegate-source");
        let destination = temp_dir("federation-delegate-destination");
        let artifact = record("federation-test-artifact", vec![string("hello")]);
        let imported = ledger::import_artifact(&source, &artifact).expect("source import");
        let resource = FederatedResource::new(
            RESOURCE_ARTIFACT,
            imported.artifact_ref.clone(),
            "molten.ledger.artifact.v1",
            "ledger-local",
            "peer:source",
        );
        let delegate = delegate_resource(&resource, "pull", "delegate:source", "delegate-root", "delegate-key")
            .expect("delegate resource");
        let inventory = inventory_for_resources_with_delegates(&InventoryWithDelegatesInput {
            peer: "peer:source",
            resources: std::slice::from_ref(&resource),
            delegates: std::slice::from_ref(&delegate),
            signer: "peer:source",
            trust_root: "root",
            key: "key",
        })
        .expect("inventory with delegate");
        let policy = FederationPullPolicy {
            required_delegate_capability: Some("pull".to_string()),
            delegate_trust_root: "delegate-root".to_string(),
            delegate_key: "delegate-key".to_string(),
            ..FederationPullPolicy::allow_all()
        };
        let pull = pull_ledger_inventory_with_policy(&PullLedgerInventoryPolicyInput {
            source_root: &source,
            dest_root: &destination,
            inventory_value: &inventory.value,
            trust_root: "root",
            key: "key",
            policy: &policy,
        })
        .expect("delegate pull");
        assert_eq!(pull.imported_refs, vec![imported.artifact_ref.clone()]);

        let no_delegate_inventory = inventory_for_resources("peer:source", &[resource], "peer:source", "root", "key")
            .expect("inventory without delegate");
        let denied_destination = temp_dir("federation-delegate-denied-destination");
        let denied = pull_ledger_inventory_with_policy(&PullLedgerInventoryPolicyInput {
            source_root: &source,
            dest_root: &denied_destination,
            inventory_value: &no_delegate_inventory.value,
            trust_root: "root",
            key: "key",
            policy: &policy,
        })
        .expect("delegate denial");
        assert!(denied.imported_refs.is_empty());
        assert_eq!(denied.denied_refs, vec![imported.artifact_ref]);
    }

    #[test]
    fn rate_limit_denies_inventory_before_fetch() {
        let source = temp_dir("federation-rate-source");
        let destination = temp_dir("federation-rate-destination");
        let first = ledger::import_artifact(&source, &record("federation-test-artifact", vec![string("one")]))
            .expect("first import");
        let second = ledger::import_artifact(&source, &record("federation-test-artifact", vec![string("two")]))
            .expect("second import");
        let inventory = inventory_ledger(&source, "peer:source", "peer:source", "root", "key").expect("inventory");
        let policy = FederationPullPolicy {
            max_resources: 1,
            ..FederationPullPolicy::allow_all()
        };
        let pull = pull_ledger_inventory_with_policy(&PullLedgerInventoryPolicyInput {
            source_root: &source,
            dest_root: &destination,
            inventory_value: &inventory.value,
            trust_root: "root",
            key: "key",
            policy: &policy,
        })
        .expect("rate limited pull");
        assert!(pull.imported_refs.is_empty());
        assert_eq!(pull.denied_refs.len(), 2);
        assert!(pull.denied_refs.contains(&first.artifact_ref));
        assert!(pull.denied_refs.contains(&second.artifact_ref));
        assert!(ledger::list_artifacts(&destination).expect("destination list").is_empty());
    }

    #[test]
    fn sync_status_assertions_capture_imports_and_denials() {
        let pull = FederationPull {
            peer: "peer:source".to_string(),
            imported_refs: vec![ref_for("imported")],
            skipped_refs: Vec::new(),
            denied_refs: vec![ref_for("denied")],
            receipt_value: federation_receipt_value(&FederationReceiptValueInput {
                operation: "test",
                decision: "fail",
                peer: "peer:source",
                resources: &[],
                imported_refs: &[],
                skipped_refs: &[],
                denied_refs: &[],
            }),
        };
        let assertions = federation_status_assertions(&pull).expect("status assertions");
        assert_eq!(assertions.len(), 3);
        assert!(assertions.iter().any(|assertion| {
            assertion.value.as_iovalue().collect_simple_record("federation-sync-status", None).is_some()
        }));
        assert!(assertions.iter().any(|assertion| {
            assertion.value.as_iovalue().collect_simple_record("federation-imported-resource", None).is_some()
        }));
        assert!(assertions.iter().any(|assertion| {
            assertion.value.as_iovalue().collect_simple_record("federation-denied-resource", None).is_some()
        }));
    }

    #[test]
    fn chunk_manifest_announcement_pulls_through_verified_chunk_store() {
        let source = temp_dir("federation-chunk-source");
        let destination = temp_dir("federation-chunk-destination");
        let iroh = temp_dir("federation-chunk-iroh");
        let put = chunk_store::put_bytes(&source, "artifact", b"abcdef", 2).expect("put chunks");
        let published =
            chunk_store::publish_iroh_blobs(&source, &iroh, &put.manifest_ref, "peer:source").expect("publish chunks");
        let resource = FederatedResource::new(
            RESOURCE_CHUNK_MANIFEST,
            put.manifest_ref.clone(),
            "molten.chunk-store.manifest.v1",
            published.ticket,
            "peer:source",
        );
        let announcement = announce_resource(&AnnounceResourceInput {
            peer: "peer:source",
            resource: &resource,
            signer: "peer:source",
            trust_root: "root",
            key: "key",
            policy_refs: &[],
        })
        .expect("announce chunk");
        let pull = pull_chunk_manifest_from_announcement(&PullChunkManifestInput {
            iroh_root: &iroh,
            dest_root: &destination,
            announcement_value: &announcement.value,
            trust_root: "root",
            key: "key",
            peer: "peer:source",
        })
        .expect("pull chunk manifest");
        assert_eq!(pull.imported_refs.first(), Some(&put.manifest_ref));
        let read = chunk_store::read_object(&destination, &put.manifest_ref).expect("read pulled chunks");
        assert_eq!(read.bytes, b"abcdef");
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_receiver_driven_sync_no_push_and_verify_before_import(tc: TestCase) {
        let count = tc.draw(generators::integers::<usize>().min_value(1).max_value(4));
        let salt = tc.draw(generators::integers::<u64>().min_value(0).max_value(1_000_000));
        let source = temp_dir("federation-hegel-source");
        let destination = temp_dir("federation-hegel-destination");
        let mut refs = Vec::with_capacity(count);
        for index in 0..count {
            let value = record("federation-hegel-artifact", vec![string(format!("{salt}-{index}"))]);
            refs.push(ledger::import_artifact(&source, &value).expect("source import").artifact_ref);
        }
        let inventory = inventory_ledger(&source, "peer:source", "peer:source", "root", "key").expect("inventory");
        assert!(ledger::list_artifacts(&destination).expect("destination before pull").is_empty());
        let wrong_key = pull_ledger_inventory(&PullLedgerInventoryInput {
            source_root: &source,
            dest_root: &destination,
            inventory_value: &inventory.value,
            trust_root: "root",
            key: "wrong-key",
            allowed_resource_types: &[],
        })
        .expect_err("wrong key fails before import");
        assert!(wrong_key.to_string().contains("signature verification failed"));
        assert!(ledger::list_artifacts(&destination).expect("destination after failed verify").is_empty());
        let pull = pull_ledger_inventory(&PullLedgerInventoryInput {
            source_root: &source,
            dest_root: &destination,
            inventory_value: &inventory.value,
            trust_root: "root",
            key: "key",
            allowed_resource_types: &[],
        })
        .expect("pull");
        assert_eq!(pull.imported_refs.len(), count);
        for reference in refs {
            assert!(pull.imported_refs.contains(&reference));
            ledger::read_artifact(&destination, &reference).expect("pulled artifact exists");
        }
    }

    #[test]
    fn receiver_policy_denial_does_not_import_remote_resource() {
        let source = temp_dir("federation-deny-source");
        let destination = temp_dir("federation-deny-destination");
        let artifact = record("federation-test-artifact", vec![string("hello")]);
        let imported = ledger::import_artifact(&source, &artifact).expect("source import");
        let inventory = inventory_ledger(&source, "peer:source", "peer:source", "root", "key").expect("inventory");
        let allowed = vec!["chain-link".to_string()];
        let pull = pull_ledger_inventory(&PullLedgerInventoryInput {
            source_root: &source,
            dest_root: &destination,
            inventory_value: &inventory.value,
            trust_root: "root",
            key: "key",
            allowed_resource_types: &allowed,
        })
        .expect("pull");
        assert!(pull.imported_refs.is_empty());
        assert_eq!(pull.denied_refs, vec![imported.artifact_ref]);
        assert!(ledger::list_artifacts(&destination).expect("destination list").is_empty());
    }

    fn ref_for(label: &str) -> String {
        canonical_hash(&record("federation-test-ref", vec![string(label)])).expect("test ref")
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);
        let nonce = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{name}-{}-{nonce}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}
