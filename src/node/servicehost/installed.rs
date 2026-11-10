//! Retained native service-host ownership for the actual node daemon lifetime.

pub struct Owner {
    host: crate::node_service_hosting::Host<
        super::authority::Current<crate::fabric_time::LiveClockAdapter>,
        super::runtime::Native,
    >,
    files: Vec<crate::node_state::Executable>,
    directories: Vec<crate::node_state::Workspace>,
    ready: preserves::IOValue,
    bundle_ref: String,
    groups: std::collections::BTreeMap<String, crate::system_extension::RecoveryGroupDeclaration>,
    observers: std::collections::BTreeMap<String, super::effects::Observer>,
}

impl Owner {
    /// An absent selector means hosting was not selected, not successful hosting.
    /// The caller must retain the returned owner across its complete daemon loop.
    pub fn open_after_source_admission(
        root: &crate::node_state::DirectoryRoot,
        gate: &crate::node_runtime::ValidatedNodeSourceGate,
    ) -> crate::error::Result<Option<Self>> {
        let selector = crate::node_state::RelativePath::parse("service-host-startup")?;
        if !root.try_exists(&selector)? {
            return Ok(None);
        }
        let bytes = root.read(&selector, 16_384)?;
        let value = crate::preserves_rail::strict_canonical_decode(&bytes)?.value;
        let fields = crate::preserves_rail::simple_record_fields(&value, "node-service-host-startup-v1", 2)?;
        let bundle_path = crate::preserves_rail::required_string_field(&fields[0], "startup admission bundle locator")?;
        let logical_tick = fields[1]
            .as_u64()
            .ok_or_else(|| super::invalid("startup logical tick requires u64"))?
            .map_err(|_| super::invalid("startup logical tick exceeds u64"))?;
        Self::open(root, gate, &bundle_path, logical_tick).map(Some)
    }

    fn open(
        root: &crate::node_state::DirectoryRoot,
        gate: &crate::node_runtime::ValidatedNodeSourceGate,
        bundle_path: &str,
        logical_tick: u64,
    ) -> crate::error::Result<Self> {
        let bytes =
            root.read(&crate::node_state::RelativePath::parse(bundle_path)?, super::codec::MAX_BUNDLE_BYTES as u64)?;
        let bundle_ref = crate::preserves_rail::content_ref_from_bytes(&bytes);
        let bundle = super::codec::decode(&bytes, &bundle_ref)?;
        let clock = crate::world_witness::live_clock()?;
        let mut authority = super::authority::Current::open(root, &bundle_ref, clock)?;
        crate::node_service_hosting::AuthorityPort::admit(
            &mut authority,
            &super::command::load_request(&bundle_ref, gate.validation_ref(), logical_tick)?,
        )?;
        let mut materialized = super::runtime::materialize(root, bundle, gate)?;
        let mut observers = std::collections::BTreeMap::new();
        super::effects::install(super::effects::InstallInput {
            root,
            bundle_ref: &bundle_ref,
            gate,
            logical_tick,
            runtime: &mut materialized.runtime,
            observers: Some(&mut observers),
        })?;
        let mut host = crate::node_service_hosting::Host::open(root, authority, materialized.runtime)?;
        let status = host.status(logical_tick)?;
        let ready = crate::preserves_rail::record("node-service-host-ready-v1", vec![
            crate::preserves_rail::string(&bundle_ref),
            crate::preserves_rail::string(gate.validation_ref()),
            crate::preserves_rail::u64_value(logical_tick),
            status,
        ]);
        Ok(Self {
            host,
            files: materialized.files,
            directories: materialized.directories,
            ready,
            bundle_ref,
            groups: materialized.groups,
            observers,
        })
    }

    /// This observation was produced only after current status authorization.
    /// A caller may publish it as the daemon's hosting readiness observation.
    pub fn ready(&self) -> &preserves::IOValue {
        &self.ready
    }

    /// A fresh observation requires fresh exact current-operation authority.
    pub fn status(&mut self, logical_tick: u64) -> crate::error::Result<preserves::IOValue> {
        self.host.status(logical_tick)
    }

    /// Pure exact current request for the independent signing producer.
    pub fn preview(
        &self,
        operation: &super::operation::Invocation,
        logical_tick: u64,
    ) -> crate::error::Result<crate::node_service_hosting::OperationRequest> {
        self.host.preview(super::command::hosting_input(operation, &self.groups)?, logical_tick)
    }

    /// Project metadata-only authority for an independent current operator signer.
    pub fn preview_scope(
        &self,
        service: &str,
        read: &crate::collaboration_scope::RequestRef,
        logical_tick: u64,
    ) -> crate::error::Result<crate::node_service_hosting::OperationRequest> {
        let observer = self.observers.get(service).ok_or_else(|| super::invalid("scope observer unavailable"))?;
        let actual = self.host.active_instance(service, logical_tick)?;
        if actual.manifest_ref != observer.manifest_ref || actual.lifecycle.generation != observer.generation {
            return Err(super::invalid("scope effect runtime is stale, uncertain, or bound to another native service"));
        }
        let input_ref = crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
            "node-service-host-scope-observation-input-v1",
            vec![
                crate::preserves_rail::string(service),
                crate::preserves_rail::string(&observer.config_ref),
                crate::preserves_rail::string(&observer.binding_ref),
                crate::preserves_rail::string(&actual.instance_id),
                crate::preserves_rail::u64_value(actual.lifecycle.generation),
                crate::preserves_rail::u64_value(actual.lifecycle.incarnation),
                crate::preserves_rail::string(read.as_str()),
            ],
        ))?;
        self.host.request("scope-observe", service, &input_ref, logical_tick)
    }

    /// Observe the retained quorum without granting effect or CurrentState authority.
    pub fn observe_scope(
        &mut self,
        service: &str,
        read: &crate::collaboration_scope::RequestRef,
        logical_tick: u64,
    ) -> crate::error::Result<preserves::IOValue> {
        let request = self.preview_scope(service, read, logical_tick)?;
        let admission = self.host.admit("scope-observe", service, &request.input_ref, logical_tick)?;
        let observer = self.observers.get(service).ok_or_else(|| super::invalid("scope observer unavailable"))?;
        let observation = observer.client.observe(read.clone())?;
        let outcome = crate::preserves_rail::record("node-service-host-scope-observed-v1", vec![
            crate::preserves_rail::string(service),
            crate::preserves_rail::string(read.as_str()),
            crate::preserves_rail::string(&observer.config_ref),
            crate::preserves_rail::string(admission),
            observation.value(),
        ]);
        Ok(crate::preserves_rail::record("node-service-host-receipt-v1", vec![
            crate::preserves_rail::string(&request.operation),
            crate::preserves_rail::string(&self.bundle_ref),
            crate::preserves_rail::u64_value(logical_tick),
            crate::preserves_rail::string(super::authority::request_ref(&self.bundle_ref, &request)?),
            outcome,
        ]))
    }

    /// Execute against the already retained actual native/scope runtimes.
    /// Every operation still passes the owning Host's exact current authority.
    // r[impl molten.system_extension.native_effect_consumer]
    pub fn execute(
        &mut self,
        operation: super::operation::Invocation,
        logical_tick: u64,
    ) -> crate::error::Result<preserves::IOValue> {
        let request = self.preview(&operation, logical_tick)?;
        let outcome = super::command::execute(&mut self.host, operation, &self.groups, logical_tick)?;
        Ok(crate::preserves_rail::record("node-service-host-receipt-v1", vec![
            crate::preserves_rail::string(&request.operation),
            crate::preserves_rail::string(&self.bundle_ref),
            crate::preserves_rail::u64_value(logical_tick),
            crate::preserves_rail::string(super::authority::request_ref(&self.bundle_ref, &request)?),
            outcome,
        ]))
    }

    /// Bounded canonical operation boundary for an admitted daemon/application caller.
    pub fn execute_packed(
        &mut self,
        command: &str,
        bytes: &[u8],
        logical_tick: u64,
    ) -> crate::error::Result<preserves::IOValue> {
        self.execute(super::operation::decode(bytes, command, logical_tick)?, logical_tick)
    }

    /// Explicitly release host/journal locks before measured execution handles.
    pub fn close(self) {
        drop(self.host);
        drop(self.files);
        drop(self.directories);
    }
}
