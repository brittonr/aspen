
impl<'a, 'b> PullEnv<'a, 'b> {
    fn new(input: &'a PullLedgerInventoryPolicyInput<'b>, inventory: &'a Inventory) -> Result<Self> {
        let existing = ledger::list_artifacts(input.dest_root)?
            .into_iter()
            .map(|entry| entry.artifact_ref)
            .collect::<BtreeSet<_>>();
        ensure_count_at_most(inventory.resources.len(), MAX_RESOURCES, "federation inventory resources")?;
        let allowed = input.policy.allowed_resource_types.iter().cloned().collect::<BtreeSet<_>>();
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

    fn apply_resource(&self, refs: &mut PullRefs, resource: &Resource) -> Result<()> {
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

    fn is_type_denied(&self, resource: &Resource) -> bool {
        !self.allowed.is_empty() && !self.allowed.contains(&resource.resource_type)
    }

    fn is_delegate_missing(&self, resource: &Resource) -> Result<bool> {
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

    fn is_duplicate(&self, refs: &PullRefs, resource: &Resource) -> bool {
        self.existing.contains(&resource.resource_ref)
            || refs.imported_refs.iter().any(|imported| imported == &resource.resource_ref)
    }

    fn import_verified(&self, refs: &mut PullRefs, resource: &Resource) -> Result<()> {
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

fn oversized_pull(inventory: &Inventory) -> Pull {
    let denied_refs = inventory.resources.iter().map(|resource| resource.resource_ref.clone()).collect::<Vec<_>>();
    Pull {
        peer: inventory.peer.clone(),
        imported_refs: Vec::new(),
        skipped_refs: Vec::new(),
        denied_refs: denied_refs.clone(),
        receipt_value: receipt_value(&ReceiptValueInput {
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

fn finish_pull(inventory: Inventory, refs: PullRefs) -> Pull {
    let decision = if refs.denied_refs.is_empty() { "pass" } else { "fail" };
    let receipt_value = receipt_value(&ReceiptValueInput {
        operation: "pull-ledger-inventory",
        decision,
        peer: &inventory.peer,
        resources: &inventory.resources,
        imported_refs: &refs.imported_refs,
        skipped_refs: &refs.skipped_refs,
        denied_refs: &refs.denied_refs,
    });
    Pull {
        peer: inventory.peer,
        imported_refs: refs.imported_refs,
        skipped_refs: refs.skipped_refs,
        denied_refs: refs.denied_refs,
        receipt_value,
    }
}

pub fn pull_chunk_manifest_from_announcement(input: &PullChunkManifestInput<'_>) -> Result<Pull> {
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
    let receipt_value = receipt_value(&ReceiptValueInput {
        operation: "pull-chunk-manifest",
        decision: "pass",
        peer: &announcement.peer,
        resources: &resources,
        imported_refs: &imported_refs,
        skipped_refs: &[],
        denied_refs: &[],
    });
    Ok(Pull {
        peer: announcement.peer,
        imported_refs,
        skipped_refs: Vec::new(),
        denied_refs: Vec::new(),
        receipt_value,
    })
}

pub fn status_assertions(pull: &Pull) -> Result<Vec<RuntimeAssertion>> {
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
        MAX_ASSERTIONS,
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
            MAX_ASSERTIONS,
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
            MAX_ASSERTIONS,
            "federation status assertions",
        )?;
    }
    Ok(assertions)
}

fn has_valid_delegate(
    delegates: &[Delegate],
    resource: &Resource,
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

pub fn receipt_value(input: &ReceiptValueInput<'_>) -> IoValue {
    record("federation-receipt-v1", vec![
        string(RECEIPT_SCHEMA),
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

fn announcement_payload_value(peer: &str, resource: &Resource, policy_refs: &[String]) -> IoValue {
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

fn inventory_payload_value(peer: &str, resources: &[Resource], delegates: &[Delegate]) -> IoValue {
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

fn resource_value(resource: &Resource) -> IoValue {
    record("federated-resource", vec![
        record("type", vec![string(&resource.resource_type)]),
        record("ref", vec![string(&resource.resource_ref)]),
        record("schema", vec![string(&resource.schema)]),
        record("transport", vec![string(&resource.transport)]),
        record("source-peer", vec![string(&resource.source_peer)]),
    ])
}

fn parse_announcement_payload(value: &IoValue) -> Result<(String, Resource, Vec<String>)> {
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
