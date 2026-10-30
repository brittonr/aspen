use crate::support::*;

impl super::twin::Live {
    pub(super) fn install_groups(&mut self) -> TestResult<()> {
        self.initialize_node()?;
        let groups = self
            .bundle
            .services
            .iter()
            .filter_map(|service| service.manifest.recovery_group.as_ref())
            .map(|group| group.group_id.clone())
            .collect::<std::collections::BTreeSet<_>>();
        for group in groups {
            self.require("install", super::wire::operation("install", vec![molten::preserves_rail::string(group)]), 1)?;
        }
        let snapshot = self.status(2)?;
        if !snapshot.members.contains_key(&self.definitions.sibling_facade.service_id) {
            return Err("live cohort must admit an independent sibling in its own singleton group".into());
        }
        Ok(())
    }

    pub(super) fn status(&mut self, tick: u64) -> TestResult<super::wire::Snapshot> {
        let result = self.require("status", super::wire::operation("status", Vec::new()), tick)?;
        super::wire::Snapshot::parse(&super::wire::outcome(&result)?)
    }

    pub(super) fn submit_work(
        &mut self,
        member: &str,
        class: molten::system_extension::WorkClass,
        tick: u64,
        label: &str,
    ) -> TestResult<(molten::system_extension::WorkPresentation, molten::system_extension::NativeCallbackValue)> {
        let snapshot = self.status(tick)?;
        let active = snapshot.members.get(member).or_fail("active work member")?;
        let bytes = molten::preserves_rail::canonical_bytes(&molten::preserves_rail::record(
            "public-node-twin-payload-v1",
            vec![
                molten::preserves_rail::string(member),
                molten::preserves_rail::string(label),
            ],
        ))?;
        let payload = molten::system_extension::NativeCallbackValue {
            value_ref: molten::preserves_rail::content_ref_from_bytes(&bytes),
            bytes,
        };
        let work_ref =
            molten::preserves_rail::canonical_hash(&molten::preserves_rail::record("public-node-twin-work-v1", vec![
                molten::preserves_rail::string(member),
                molten::preserves_rail::string(label),
                molten::preserves_rail::string(&payload.value_ref),
            ]))?;
        let work = molten::system_extension::WorkPresentation {
            work_ref,
            class,
            generation: active.generation,
            incarnation: active.incarnation,
        };
        self.require(
            "submit",
            super::wire::operation("submit", vec![
                molten::preserves_rail::string(member),
                super::wire::work(&work),
                super::wire::payload(&payload),
            ]),
            tick,
        )?;
        Ok((work, payload))
    }

    pub(super) fn readmit_work(&mut self, member: &str, work_ref: &str, tick: u64) -> TestResult<String> {
        let result = self.require(
            "readmit",
            super::wire::operation("readmit", vec![
                molten::preserves_rail::string(member),
                molten::preserves_rail::string(work_ref),
            ]),
            tick,
        )?;
        let value = super::wire::outcome(&result)?;
        let fields = molten::preserves_rail::simple_record_fields(&value, "node-service-host-claim-v1", 4)?;
        super::wire::text(&fields[0])
    }

    pub(super) fn deliver_work(
        &mut self,
        work: &molten::system_extension::WorkPresentation,
        payload: &molten::system_extension::NativeCallbackValue,
        claim: Option<&str>,
        tick: u64,
    ) -> TestResult<bool> {
        let snapshot = self.status(tick)?;
        let member = snapshot.work.get(&work.work_ref).or_fail("retained delivery owner")?.member.as_str();
        let active = snapshot.members.get(member).or_fail("active delivery member")?;
        let service = self
            .bundle
            .services
            .iter()
            .chain(self.bundle.replacements.iter().map(|replacement| &replacement.service))
            .find(|service| service.executable.manifest_ref == active.manifest_ref)
            .or_fail("exact active admitted executable")?;
        let endpoint = molten::preserves_rail::canonical_hash(&molten::preserves_rail::record(
            "local-native-host-endpoint-v1",
            vec![
                molten::preserves_rail::string(member),
                molten::preserves_rail::string(&active.manifest_ref),
            ],
        ))?;
        let trust = molten::node_service_host::authority::wire::decode_trust(
            &self.root.read(&molten::node_state::RelativePath::parse("service-host-trust")?, 262144)?,
        )?;
        let peer =
            molten::preserves_rail::canonical_hash(&molten::preserves_rail::record("local-ucan-holder-v1", vec![
                molten::preserves_rail::string(&trust.holder),
            ]))?;
        let request_ref =
            molten::preserves_rail::canonical_hash(&molten::preserves_rail::record("public-node-delivery-v1", vec![
                molten::preserves_rail::string(&work.work_ref),
                molten::preserves_rail::u64_value(tick),
                molten::preserves_rail::sequence(claim.into_iter().map(molten::preserves_rail::string).collect()),
            ]))?;
        let ingress = molten::preserves_rail::record("node-service-host-ingress-v1", vec![
            molten::preserves_rail::string(request_ref),
            molten::preserves_rail::string(endpoint),
            molten::preserves_rail::string(peer),
            molten::preserves_rail::string(member),
            molten::preserves_rail::string(&active.manifest_ref),
            molten::preserves_rail::u64_value(active.generation),
            molten::preserves_rail::string(&service.executable.authority_ref),
            molten::preserves_rail::string(&service.executable.policy_ref),
            molten::preserves_rail::string(&service.executable.resource_ref),
            molten::preserves_rail::string(&service.host_profile.transport_profile_ref),
            super::wire::payload(payload),
            molten::preserves_rail::u64_value(payload.bytes.len() as u64),
            molten::preserves_rail::string("materialized-native-v3"),
        ]);
        let delivery = molten::preserves_rail::record("node-service-host-delivery-v1", vec![
            molten::preserves_rail::sequence(claim.into_iter().map(molten::preserves_rail::string).collect()),
            ingress,
        ]);
        Ok(self
            .execute(
                "deliver",
                super::wire::operation("deliver", vec![
                    molten::preserves_rail::string(member),
                    super::wire::work(work),
                    delivery,
                ]),
                tick,
            )?
            .0)
    }

    pub(super) fn progress_sibling(&mut self, tick: u64) -> TestResult<bool> {
        let sibling = self.definitions.sibling_facade.service_id.clone();
        let before = self.status(tick)?.members.get(&sibling).or_fail("sibling before work")?.sequence;
        let (work, payload) = self.submit_work(
            &sibling,
            molten::system_extension::WorkClass::RuntimeLocal,
            tick,
            &format!("progress-{tick}"),
        )?;
        let was_accepted = self.deliver_work(&work, &payload, None, tick)?;
        let after = self.status(tick)?;
        Ok(was_accepted
            && after.members.get(&sibling).or_fail("sibling after work")?.sequence > before
            && after.work.get(&work.work_ref).is_some_and(|work| work.receipt_ref.is_some()))
    }

    pub(super) fn supervise_group(&mut self, tick: u64) -> TestResult<bool> {
        let group = self.definitions.group.group_id.clone();
        let member = self.definitions.group.members[0].member_id.clone();
        Ok(self
            .execute(
                "supervise",
                super::wire::operation("supervise", vec![
                    molten::preserves_rail::string(group),
                    molten::preserves_rail::string(member),
                    molten::preserves_rail::bool_value(false),
                    molten::preserves_rail::string("retryable"),
                ]),
                tick,
            )?
            .0)
    }

    pub(super) fn materialize(
        &self,
        service: &molten::node_service_host::codec::ExtensionInput,
        reference: &str,
    ) -> TestResult<Vec<u8>> {
        let scope = self
            .root
            .ledger()?
            .open_subdir(&molten::node_state::RelativePath::parse(&service.cache_namespace)?)?;
        let mut port = molten::system_extension::CapabilityCallbackValuePort::new(scope)?;
        let value =
            molten::system_extension::NativeCallbackValuePort::materialize(&mut port, reference, MAX_VALUE_BYTES)
                .or_fail("actual durable callback value")?;
        Ok(value.bytes)
    }

    pub(super) fn migration_ref(
        &self,
        service: &molten::node_service_host::codec::ExtensionInput,
        generation: u64,
    ) -> TestResult<String> {
        let scope = self
            .root
            .ledger()?
            .open_subdir(&molten::node_state::RelativePath::parse(&service.cache_namespace)?)?
            .open_subdir(&molten::node_state::RelativePath::parse("values")?)?;
        for entry in scope.list_entries()? {
            let bytes = scope.read_entry(&entry, MAX_VALUE_BYTES)?;
            let Ok(value) = molten::preserves_rail::parse_canonical_bytes(&bytes) else {
                continue;
            };
            let Some(fields) = value.collect_simple_record("system-extension-state-migration-v1", Some(11)) else {
                continue;
            };
            let generation_field = molten::preserves_rail::value_to_iovalue(&fields[9]);
            let generation_field = molten::preserves_rail::simple_record_fields(&generation_field, "generation", 1)?;
            if super::wire::number(&generation_field[0])? == generation {
                return Ok(molten::preserves_rail::canonical_hash(&value)?);
            }
        }
        Err("actual native migration receipt was not materially published".into())
    }
}
