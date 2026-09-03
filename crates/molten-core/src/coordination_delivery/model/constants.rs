pub const DELIVERY_MANIFEST_SCHEMA: &str = "molten.coordination-delivery-manifest.v1";
pub const DELIVERY_POLICY_SCHEMA: &str = "molten.coordination-delivery-policy.v1";
pub const DELIVERY_STATE_SCHEMA: &str = "molten.coordination-delivery-state.v1";
pub const DELIVERY_REQUEST_SCHEMA: &str = "molten.coordination-delivery-request.v1";
pub const DELIVERY_TRANSITION_SCHEMA: &str = "molten.coordination-delivery-transition.v1";
pub const DELIVERY_STATUS_SCHEMA: &str = "molten.coordination-delivery-status.v1";
pub const DELIVERY_WORKER_PLAN_SCHEMA: &str = "molten.coordination-delivery-worker-plan.v1";

pub const PORT_CONSISTENCY: &str = "fabric_consistency";
pub const PORT_DURABLE_STATE: &str = "fabric_durable_state";
pub const PORT_LOGICAL_TIME: &str = "fabric_logical_time";
pub const PORT_RESOURCE: &str = "fabric_resource";
pub const PORT_OBSERVABILITY: &str = "fabric_observability";

pub const REQUIRED_DELIVERY_PORT_COUNT: usize = 5;
pub const REQUIRED_DELIVERY_PORTS: [&str; REQUIRED_DELIVERY_PORT_COUNT] = [
    PORT_CONSISTENCY,
    PORT_DURABLE_STATE,
    PORT_LOGICAL_TIME,
    PORT_RESOURCE,
    PORT_OBSERVABILITY,
];

pub const REQUIRED_DELIVERY_NON_CLAIM_COUNT: usize = 6;
pub const REQUIRED_DELIVERY_NON_CLAIMS: [&str; REQUIRED_DELIVERY_NON_CLAIM_COUNT] = [
    "delivery-evidence-does-not-grant-authority",
    "delivery-evidence-does-not-prove-exactly-once-effects",
    "delivery-evidence-does-not-prove-payload-correctness",
    "delivery-evidence-does-not-prove-global-ordering",
    "delivery-evidence-does-not-prove-store-or-broker-correctness",
    "delivery-evidence-does-not-establish-release-eligibility",
];

pub const MAX_DELIVERY_ID_BYTES: usize = 192;
pub const MAX_DELIVERY_REF_BYTES: usize = 160;
pub const MAX_DELIVERY_CLASS_BYTES: usize = 96;
pub const MAX_DELIVERY_COLLECTION_ITEMS: u32 = 4_096;
pub const MAX_DELIVERY_METADATA_BYTES: u32 = 65_536;
pub const MAX_DELIVERY_ATTEMPTS: u64 = 64;
pub const MAX_DELIVERY_TICKS: u64 = 1_000_000_000_000;
pub const MAX_DELIVERY_STATUS_ITEMS: u32 = 256;
pub const INITIAL_DELIVERY_REVISION: u64 = 0;
pub const INITIAL_DELIVERY_SEQUENCE: u64 = 1;
pub const INITIAL_DELIVERY_FENCING_TOKEN: u64 = 1;
pub const INITIAL_DELIVERY_CYCLE: u32 = 1;
