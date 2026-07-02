type IoValue = preserves::IOValue;

type Result<T> = crate::error::Result<T>;

const RUNTIME_VAT_AMBIENT_AUTHORITY_FIXTURE_SCHEMA: &str =
    crate::preserves_rail::RUNTIME_VAT_AMBIENT_AUTHORITY_FIXTURE_SCHEMA;
const RUNTIME_VAT_AUTHORITY_GRAPH_FIXTURE_SCHEMA: &str =
    crate::preserves_rail::RUNTIME_VAT_AUTHORITY_GRAPH_FIXTURE_SCHEMA;
const RUNTIME_VAT_DISTRIBUTED_REF_FIXTURE_SCHEMA: &str =
    crate::preserves_rail::RUNTIME_VAT_DISTRIBUTED_REF_FIXTURE_SCHEMA;
const RUNTIME_VAT_FIXTURE_RUN_SCHEMA: &str = crate::preserves_rail::RUNTIME_VAT_FIXTURE_RUN_SCHEMA;
const RUNTIME_VAT_OBJECT_REF_SCHEMA: &str = crate::preserves_rail::RUNTIME_VAT_OBJECT_REF_SCHEMA;
const RUNTIME_VAT_OBJECT_UPGRADE_RECIPE_SCHEMA: &str = crate::preserves_rail::RUNTIME_VAT_OBJECT_UPGRADE_RECIPE_SCHEMA;
const RUNTIME_VAT_PORTABLE_STORAGE_FIXTURE_SCHEMA: &str =
    crate::preserves_rail::RUNTIME_VAT_PORTABLE_STORAGE_FIXTURE_SCHEMA;
const RUNTIME_VAT_PROMISE_FIXTURE_SCHEMA: &str = crate::preserves_rail::RUNTIME_VAT_PROMISE_FIXTURE_SCHEMA;
const RUNTIME_VAT_REPLAY_FIXTURE_SCHEMA: &str = crate::preserves_rail::RUNTIME_VAT_REPLAY_FIXTURE_SCHEMA;
const RUNTIME_VAT_RESTORE_RECEIPT_SCHEMA: &str = crate::preserves_rail::RUNTIME_VAT_RESTORE_RECEIPT_SCHEMA;
const RUNTIME_VAT_RIGHTS_FIXTURE_SCHEMA: &str = crate::preserves_rail::RUNTIME_VAT_RIGHTS_FIXTURE_SCHEMA;
const RUNTIME_VAT_SNAPSHOT_SCHEMA: &str = crate::preserves_rail::RUNTIME_VAT_SNAPSHOT_SCHEMA;
const RUNTIME_VAT_TIME_TRAVEL_FIXTURE_SCHEMA: &str = crate::preserves_rail::RUNTIME_VAT_TIME_TRAVEL_FIXTURE_SCHEMA;

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value)
}

#[cfg(test)]
fn to_text(value: &IoValue) -> Result<String> {
    crate::preserves_rail::to_text(value)
}

const LOCAL_VAT_ID: &str = "vat:fixture:local";
const REMOTE_VAT_ID: &str = "vat:fixture:remote";
const ROOT_OBJECT_ID: &str = "object:root";
const HELPER_OBJECT_ID: &str = "object:helper";
const SPAWNED_OBJECT_ID: &str = "object:spawned";
const FAR_OBJECT_ID: &str = "object:remote";
const PROXY_OBJECT_ID: &str = "object:proxy";
const PIPELINE_MAX_QUEUE: u64 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VatReferenceKind {
    Near,
    Far,
    Proxy,
}

impl VatReferenceKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Near => "near",
            Self::Far => "far",
            Self::Proxy => "proxy",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VatObjectRef {
    pub vat_id: String,
    pub object_id: String,
    pub kind: VatReferenceKind,
    pub authority_refs: Vec<String>,
}

impl VatObjectRef {
    pub fn new(
        vat_id: impl Into<String>,
        object_id: impl Into<String>,
        kind: VatReferenceKind,
        authority_refs: Vec<String>,
    ) -> Self {
        let mut sorted_authority_refs = authority_refs;
        sorted_authority_refs.sort();
        sorted_authority_refs.dedup();
        Self {
            vat_id: vat_id.into(),
            object_id: object_id.into(),
            kind,
            authority_refs: sorted_authority_refs,
        }
    }

    pub fn value(&self) -> IoValue {
        record("vat-object-ref-v1", vec![
            string(RUNTIME_VAT_OBJECT_REF_SCHEMA),
            string(&self.vat_id),
            string(&self.object_id),
            string(self.kind.as_str()),
            sequence(self.authority_refs.iter().map(string).collect()),
        ])
    }

    pub fn object_ref(&self) -> Result<String> {
        canonical_hash(&self.value())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VatCallEvidence {
    pub name: String,
    pub receipt: crate::runtime::RuntimePredicateReceipt,
}

impl VatCallEvidence {
    fn value(&self) -> IoValue {
        record("vat-call-evidence-v1", vec![
            string(&self.name),
            record("receipt-ref", vec![string(&self.receipt.receipt_ref)]),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VatFixtureRun {
    pub value: IoValue,
    pub run_ref: String,
    pub receipts: Vec<crate::runtime::RuntimePredicateReceipt>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VatSnapshotFixture {
    pub value: IoValue,
    pub snapshot_ref: String,
    pub fixture_ref: String,
    pub receipts: Vec<crate::runtime::RuntimePredicateReceipt>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VatRestoreFixture {
    pub value: IoValue,
    pub fixture_ref: String,
    pub receipts: Vec<IoValue>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VatPromiseFixture {
    pub value: IoValue,
    pub fixture_ref: String,
    pub receipts: Vec<crate::runtime::RuntimePredicateReceipt>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VatAmbientAuthorityFixture {
    pub value: IoValue,
    pub fixture_ref: String,
    pub receipts: Vec<crate::runtime::RuntimePredicateReceipt>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VatRightsFixture {
    pub value: IoValue,
    pub fixture_ref: String,
    pub receipts: Vec<crate::runtime::RuntimePredicateReceipt>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VatDistributedRefFixture {
    pub value: IoValue,
    pub fixture_ref: String,
    pub receipts: Vec<crate::runtime::RuntimePredicateReceipt>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VatDebugFixture {
    pub value: IoValue,
    pub fixture_ref: String,
    pub receipts: Vec<IoValue>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VatReplayFixture {
    pub value: IoValue,
    pub fixture_ref: String,
    pub receipts: Vec<IoValue>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VatReplayRun {
    value: IoValue,
    run_ref: String,
    trace_ref: String,
    effect_request_ref: String,
    effect_response_ref: String,
    random_request_ref: String,
    random_response_ref: String,
    policy_decision_ref: String,
    final_state_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VatReplayDivergenceKind {
    None,
    Input,
    EffectRequest,
    EffectResponse,
    PolicyDecision,
    StateHash,
}

impl VatReplayDivergenceKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Input => "input",
            Self::EffectRequest => "effect-request",
            Self::EffectResponse => "effect-response",
            Self::PolicyDecision => "policy-decision",
            Self::StateHash => "state-hash",
        }
    }
}

struct FixtureObjects {
    root: VatObjectRef,
    helper: VatObjectRef,
    spawned: VatObjectRef,
    far: VatObjectRef,
    proxy: VatObjectRef,
    root_ref: String,
    helper_ref: String,
    spawned_ref: String,
    far_ref: String,
    proxy_ref: String,
}

impl FixtureObjects {
    fn object_values(&self) -> Vec<IoValue> {
        [&self.root, &self.helper, &self.spawned, &self.far, &self.proxy]
            .iter()
            .map(|object| object.value())
            .collect()
    }
}

fn fixture_objects() -> Result<FixtureObjects> {
    let root = VatObjectRef::new(LOCAL_VAT_ID, ROOT_OBJECT_ID, VatReferenceKind::Near, Vec::new());
    let helper = VatObjectRef::new(LOCAL_VAT_ID, HELPER_OBJECT_ID, VatReferenceKind::Near, vec![root.object_ref()?]);
    let spawned =
        VatObjectRef::new(LOCAL_VAT_ID, SPAWNED_OBJECT_ID, VatReferenceKind::Near, vec![helper.object_ref()?]);
    let far = VatObjectRef::new(REMOTE_VAT_ID, FAR_OBJECT_ID, VatReferenceKind::Far, Vec::new());
    let proxy = VatObjectRef::new(LOCAL_VAT_ID, PROXY_OBJECT_ID, VatReferenceKind::Proxy, vec![helper.object_ref()?]);

    Ok(FixtureObjects {
        root_ref: root.object_ref()?,
        helper_ref: helper.object_ref()?,
        spawned_ref: spawned.object_ref()?,
        far_ref: far.object_ref()?,
        proxy_ref: proxy.object_ref()?,
        root,
        helper,
        spawned,
        far,
        proxy,
    })
}
