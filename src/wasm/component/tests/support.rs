use super::super::*;

const FIXED_MEMORY_BYTES: u64 = 65_536;
const FIXED_MEMORY_LIMITS: &str = "1 1";
const GROWING_MEMORY_LIMITS: &str = "1 2";
const OVER_LIMIT_MEMORY_PAGES: u64 = 257;
const OVER_LIMIT_MEMORY_LIMITS: &str = "257 257";
const FIXED_TABLE_ELEMENTS: u64 = 0;
const FIXED_INSTANCE_COUNT: u64 = 1;
const FIXED_MEMORY_COUNT: u64 = 1;
const FIXED_TABLE_COUNT: u64 = 0;
const FIXTURE_INVALID_OUTPUT_ADDRESS: i32 = 1_024;
const FIXTURE_INVALID_OUTPUT_LENGTH: i32 = 1;
const FIXTURE_RESULT_ADDRESS: i32 = 2_048;
const FIXTURE_RESULT_POINTER_ADDRESS: i32 = 2_052;
const FIXTURE_RESULT_LENGTH_ADDRESS: i32 = 2_056;
const FIXTURE_HEAP_ADDRESS: i32 = 4_096;

pub struct Fixture {
    pub profile: RuntimeProfile,
    pub component_bytes: Vec<u8>,
    pub wit_bytes: Vec<u8>,
    pub bundle: MantleBundle,
    pub envelope: AdmissionEnvelope,
    pub facts: ArtifactFacts,
    pub import_manifest: DeclaredManifest,
}

impl Fixture {
    pub fn new(consumer: Consumer) -> Self {
        Self::with_profile(consumer, supported().expect("supported component profile"))
    }

    pub fn with_profile(consumer: Consumer, profile: RuntimeProfile) -> Self {
        let component_bytes = identity_bytes();
        let wit_bytes =
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/wit/molten-component-runtime/runtime.wit")).to_vec();
        let component = MaterializedObjectIdentity::measure(&component_bytes).expect("component identity");
        let wit = MaterializedObjectIdentity::measure(&wit_bytes).expect("WIT identity");
        let mut bundle = MantleBundle {
            schema_id: MANTLE_COMPONENT_BUNDLE_SCHEMA.to_string(),
            bundle_ref: String::new(),
            component,
            wit,
            artifact_kind: ArtifactKind::Component,
            consumer,
            expected_profile_id: profile.profile_id.clone(),
            expected_cohort_ref: r#ref(&profile),
            build_cohort_ref: fixture_ref("mantle-build-cohort"),
            octet_report_ref: fixture_ref("octet-report"),
            stage_receipt_refs: vec![fixture_ref("mantle-stage")],
            embedded_admission_refs: Vec::new(),
            has_portable_bytes: true,
            has_precompiled_bytes: false,
        };
        bundle.bundle_ref = mantle_bundle_ref(&bundle);
        let envelope = AdmissionEnvelope {
            schema_id: COMPONENT_ADMISSION_ENVELOPE_SCHEMA.to_string(),
            bundle_ref: bundle.bundle_ref.clone(),
            valence_sidecar_refs: vec![fixture_ref("valence-sidecar")],
            cairn_acceptance_refs: vec![fixture_ref("cairn-acceptance")],
            policy_refs: vec![fixture_ref("policy")],
            authority_refs: vec![fixture_ref("authority")],
            resource_refs: vec![fixture_ref("resource")],
        };
        let facts = valid_facts(&profile);
        let import_manifest = valid_import_manifest(&profile);
        Self {
            profile,
            component_bytes,
            wit_bytes,
            bundle,
            envelope,
            facts,
            import_manifest,
        }
    }

    pub fn replace_component_bytes(&mut self, component_bytes: Vec<u8>) {
        self.component_bytes = component_bytes;
        self.bundle.component = MaterializedObjectIdentity::measure(&self.component_bytes).expect("component identity");
        self.bundle.bundle_ref = mantle_bundle_ref(&self.bundle);
        self.envelope.bundle_ref = self.bundle.bundle_ref.clone();
    }

    pub fn source(&self) -> ArtifactSource<'_> {
        ArtifactSource::Mantle {
            bundle: &self.bundle,
            envelope: &self.envelope,
            component_bytes: &self.component_bytes,
            wit_bytes: &self.wit_bytes,
        }
    }

    /// A time-sliced execution request for this fixture.
    pub fn sliced_request<'a>(&'a self, input: &'a preserves::IOValue) -> ExecutionRequest<'a> {
        ExecutionRequest {
            requested_profile: RequestedExecutionProfile::TimeSlicedComponentV1,
            ..self.request(input)
        }
    }

    pub fn request<'a>(&'a self, input: &'a preserves::IOValue) -> ExecutionRequest<'a> {
        ExecutionRequest {
            profile: &self.profile,
            requested_profile: RequestedExecutionProfile::ComponentV1,
            evidence_scope: EvidenceScope::Production,
            source: self.source(),
            facts: &self.facts,
            import_manifest: &self.import_manifest,
            import_grants: &[],
            input,
        }
    }
}

pub fn valid_facts(profile: &RuntimeProfile) -> ArtifactFacts {
    ArtifactFacts {
        artifact_kind: ArtifactKind::Component,
        declared_profile_id: profile.profile_id.clone(),
        declared_cohort_ref: r#ref(profile),
        declared_world: profile.wit.world.clone(),
        imports: Vec::new(),
        exports: vec![COMPONENT_INVOKE_EXPORT.to_string()],
        enabled_features: vec![
            "bulk-memory".to_string(),
            "component-model".to_string(),
            "multi-value".to_string(),
            "reference-types".to_string(),
            "simd".to_string(),
        ],
        memory: GrowthFacts {
            initial: FIXED_MEMORY_BYTES,
            maximum: Some(FIXED_MEMORY_BYTES),
            strategy: GrowthStrategy::Fixed,
        },
        table: GrowthFacts {
            initial: FIXED_TABLE_ELEMENTS,
            maximum: Some(FIXED_TABLE_ELEMENTS),
            strategy: GrowthStrategy::Fixed,
        },
        instances: FIXED_INSTANCE_COUNT,
        memories: FIXED_MEMORY_COUNT,
        tables: FIXED_TABLE_COUNT,
    }
}

pub fn valid_import_manifest(profile: &RuntimeProfile) -> DeclaredManifest {
    build_manifest(ManifestInput {
        profile_id: profile.profile_id.clone(),
        wit_package: profile.wit.package.clone(),
        world: profile.wit.world.clone(),
        imports: Vec::new(),
        exports: vec![COMPONENT_INVOKE_EXPORT.to_string()],
    })
    .expect("valid import manifest fixture")
}

pub fn input_value() -> preserves::IOValue {
    crate::preserves_rail::record("component-fixture-input", vec![crate::preserves_rail::string("ok")])
}

pub fn fixture_ref(label: &str) -> String {
    super::super::model::content_ref(label.as_bytes())
}

pub fn identity_bytes() -> Vec<u8> {
    bytes_with_guest(FIXED_MEMORY_LIMITS, "", &identity_invoke_body())
}

fn identity_invoke_body() -> String {
    format!(
        "i32.const {FIXTURE_RESULT_ADDRESS}\n\
         i32.const 0\n\
         i32.store8\n\
         i32.const {FIXTURE_RESULT_POINTER_ADDRESS}\n\
         local.get $ptr\n\
         i32.store\n\
         i32.const {FIXTURE_RESULT_LENGTH_ADDRESS}\n\
         local.get $len\n\
         i32.store\n\
         i32.const {FIXTURE_RESULT_ADDRESS}"
    )
}

/// A component that spins for `iterations` loop turns, then echoes its input.
pub fn cpu_heavy_bytes(iterations: u32) -> Vec<u8> {
    let invoke_body = format!(
        "(local $remaining i32)\n\
         i32.const {iterations}\n\
         local.set $remaining\n\
         (block $done\n\
           (loop $spin\n\
             local.get $remaining\n\
             i32.eqz\n\
             br_if $done\n\
             local.get $remaining\n\
             i32.const 1\n\
             i32.sub\n\
             local.set $remaining\n\
             br $spin))\n\
         {}",
        identity_invoke_body()
    );
    bytes_with_guest(FIXED_MEMORY_LIMITS, "", &invoke_body)
}

pub fn over_limit_memory_bytes() -> (Vec<u8>, u64) {
    let bytes = bytes_with_guest(OVER_LIMIT_MEMORY_LIMITS, "", &identity_invoke_body());
    let declared_bytes = OVER_LIMIT_MEMORY_PAGES * FIXED_MEMORY_BYTES;
    (bytes, declared_bytes)
}

pub fn invalid_output_bytes() -> Vec<u8> {
    let data = format!(r#"(data (i32.const {FIXTURE_INVALID_OUTPUT_ADDRESS}) "\ff")"#);
    let invoke_body = format!(
        "i32.const {FIXTURE_RESULT_ADDRESS}\n\
         i32.const 0\n\
         i32.store8\n\
         i32.const {FIXTURE_RESULT_POINTER_ADDRESS}\n\
         i32.const {FIXTURE_INVALID_OUTPUT_ADDRESS}\n\
         i32.store\n\
         i32.const {FIXTURE_RESULT_LENGTH_ADDRESS}\n\
         i32.const {FIXTURE_INVALID_OUTPUT_LENGTH}\n\
         i32.store\n\
         i32.const {FIXTURE_RESULT_ADDRESS}"
    );
    bytes_with_guest(FIXED_MEMORY_LIMITS, &data, &invoke_body)
}

pub fn fuel_exhaustion_bytes() -> Vec<u8> {
    bytes_with_guest(FIXED_MEMORY_LIMITS, "", "(loop $forever (br $forever))\nunreachable")
}

pub fn dynamic_growth_bytes() -> Vec<u8> {
    let invoke_body = format!("i32.const {FIXTURE_RESULT_ADDRESS}");
    bytes_with_guest(GROWING_MEMORY_LIMITS, "", &invoke_body)
}

fn bytes_with_guest(memory_limits: &str, extra_guest_items: &str, invoke_body: &str) -> Vec<u8> {
    let wat = format!(
        r#"(component
  (core module $guest
    (memory (export "memory") {memory_limits})
    {extra_guest_items}
    (global $heap (mut i32) (i32.const {FIXTURE_HEAP_ADDRESS}))
    (func $realloc (export "cabi_realloc")
      (param $old-ptr i32) (param $old-size i32) (param $align i32) (param $new-size i32)
      (result i32)
      (local $result i32)
      global.get $heap
      local.set $result
      global.get $heap
      local.get $new-size
      i32.add
      global.set $heap
      local.get $result)
    (func $invoke (export "invoke") (param $ptr i32) (param $len i32) (result i32)
      {invoke_body}))
  (core instance $guest-instance (instantiate $guest))
  (alias core export $guest-instance "memory" (core memory $memory))
  (alias core export $guest-instance "cabi_realloc" (core func $realloc))
  (alias core export $guest-instance "invoke" (core func $invoke))
  (type $payload (list u8))
  (type $outcome (result $payload (error string)))
  (type $invoke-type (func (param "input" $payload) (result $outcome)))
  (func $invoke-lifted
    (type $invoke-type)
    (canon lift (core func $invoke) (memory $memory) (realloc $realloc)))
  (export "invoke" (func $invoke-lifted)))"#
    );
    wat::parse_str(&wat).expect("component fixture WAT")
}

/// The operated product engine for a fixture's admitted profile.
pub fn engine(fixture: &Fixture) -> Engine {
    Engine::build(&fixture.profile).expect("operated component engine")
}

/// Execute one request through a fresh operated engine for its admitted profile.
pub fn execute(request: &ExecutionRequest<'_>) -> ExecutionOutcome {
    Engine::build(request.profile).expect("operated component engine").execute(request)
}
