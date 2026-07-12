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

pub struct ComponentFixture {
    pub profile: ComponentRuntimeProfile,
    pub component_bytes: Vec<u8>,
    pub wit_bytes: Vec<u8>,
    pub bundle: MantleComponentBundle,
    pub envelope: ComponentAdmissionEnvelope,
    pub facts: ComponentArtifactFacts,
}

impl ComponentFixture {
    pub fn new(consumer: ComponentConsumer) -> Self {
        let profile = supported_component_profile().expect("supported component profile");
        let component_bytes = identity_component_bytes();
        let wit_bytes =
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/wit/molten-component-runtime/runtime.wit")).to_vec();
        let component = MaterializedObjectIdentity::measure(&component_bytes).expect("component identity");
        let wit = MaterializedObjectIdentity::measure(&wit_bytes).expect("WIT identity");
        let mut bundle = MantleComponentBundle {
            schema_id: MANTLE_COMPONENT_BUNDLE_SCHEMA.to_string(),
            bundle_ref: String::new(),
            component,
            wit,
            artifact_kind: WasmArtifactKind::Component,
            consumer,
            expected_profile_id: profile.profile_id.clone(),
            expected_cohort_ref: component_profile_ref(&profile),
            build_cohort_ref: fixture_ref("mantle-build-cohort"),
            octet_report_ref: fixture_ref("octet-report"),
            stage_receipt_refs: vec![fixture_ref("mantle-stage")],
            embedded_admission_refs: Vec::new(),
            has_portable_bytes: true,
            has_precompiled_bytes: false,
        };
        bundle.bundle_ref = mantle_bundle_ref(&bundle);
        let envelope = ComponentAdmissionEnvelope {
            schema_id: COMPONENT_ADMISSION_ENVELOPE_SCHEMA.to_string(),
            bundle_ref: bundle.bundle_ref.clone(),
            valence_sidecar_refs: vec![fixture_ref("valence-sidecar")],
            cairn_acceptance_refs: vec![fixture_ref("cairn-acceptance")],
            policy_refs: vec![fixture_ref("policy")],
            authority_refs: vec![fixture_ref("authority")],
            resource_refs: vec![fixture_ref("resource")],
        };
        let facts = valid_facts(&profile);
        Self {
            profile,
            component_bytes,
            wit_bytes,
            bundle,
            envelope,
            facts,
        }
    }

    pub fn replace_component_bytes(&mut self, component_bytes: Vec<u8>) {
        self.component_bytes = component_bytes;
        self.bundle.component = MaterializedObjectIdentity::measure(&self.component_bytes).expect("component identity");
        self.bundle.bundle_ref = mantle_bundle_ref(&self.bundle);
        self.envelope.bundle_ref = self.bundle.bundle_ref.clone();
    }

    pub fn source(&self) -> ComponentArtifactSource<'_> {
        ComponentArtifactSource::Mantle {
            bundle: &self.bundle,
            envelope: &self.envelope,
            component_bytes: &self.component_bytes,
            wit_bytes: &self.wit_bytes,
        }
    }

    pub fn request<'a>(&'a self, input: &'a preserves::IOValue) -> ComponentExecutionRequest<'a> {
        ComponentExecutionRequest {
            profile: &self.profile,
            requested_profile: RequestedExecutionProfile::ComponentV1,
            evidence_scope: EvidenceScope::Production,
            source: self.source(),
            facts: &self.facts,
            import_grants: &[],
            input,
        }
    }
}

pub fn valid_facts(profile: &ComponentRuntimeProfile) -> ComponentArtifactFacts {
    ComponentArtifactFacts {
        artifact_kind: WasmArtifactKind::Component,
        declared_profile_id: profile.profile_id.clone(),
        declared_cohort_ref: component_profile_ref(profile),
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
        memory: ComponentGrowthFacts {
            initial: FIXED_MEMORY_BYTES,
            maximum: Some(FIXED_MEMORY_BYTES),
            strategy: GrowthStrategy::Fixed,
        },
        table: ComponentGrowthFacts {
            initial: FIXED_TABLE_ELEMENTS,
            maximum: Some(FIXED_TABLE_ELEMENTS),
            strategy: GrowthStrategy::Fixed,
        },
        instances: FIXED_INSTANCE_COUNT,
        memories: FIXED_MEMORY_COUNT,
        tables: FIXED_TABLE_COUNT,
    }
}

pub fn input_value() -> preserves::IOValue {
    crate::preserves_rail::record("component-fixture-input", vec![crate::preserves_rail::string("ok")])
}

pub fn fixture_ref(label: &str) -> String {
    super::super::model::content_ref(label.as_bytes())
}

pub fn identity_component_bytes() -> Vec<u8> {
    component_bytes_with_guest(FIXED_MEMORY_LIMITS, "", &identity_invoke_body())
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

pub fn over_limit_memory_component_bytes() -> (Vec<u8>, u64) {
    let bytes = component_bytes_with_guest(OVER_LIMIT_MEMORY_LIMITS, "", &identity_invoke_body());
    let declared_bytes = OVER_LIMIT_MEMORY_PAGES * FIXED_MEMORY_BYTES;
    (bytes, declared_bytes)
}

pub fn invalid_output_component_bytes() -> Vec<u8> {
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
    component_bytes_with_guest(FIXED_MEMORY_LIMITS, &data, &invoke_body)
}

pub fn fuel_exhaustion_component_bytes() -> Vec<u8> {
    component_bytes_with_guest(FIXED_MEMORY_LIMITS, "", "(loop $forever (br $forever))\nunreachable")
}

pub fn dynamic_growth_component_bytes() -> Vec<u8> {
    let invoke_body = format!("i32.const {FIXTURE_RESULT_ADDRESS}");
    component_bytes_with_guest(GROWING_MEMORY_LIMITS, "", &invoke_body)
}

fn component_bytes_with_guest(memory_limits: &str, extra_guest_items: &str, invoke_body: &str) -> Vec<u8> {
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
