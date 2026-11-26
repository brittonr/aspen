type IoValue = preserves::IOValue;

use crate::bounded::PushLimited;
type OrderedMap<K, V> = std::collections::BTreeMap<K, V>;
type OrderedSet<T> = std::collections::BTreeSet<T>;
type Record<T> = preserves::Record<T>;
type Value<T> = preserves::Value<T>;
type MoltenError = crate::error::Failure;
type Result<T> = crate::error::Result<T>;
type ControlRegistryRuntime = crate::raft_control_plane::ControlRegistryRuntime;
type RaftReadReceipt = crate::raft_control_plane::RaftReadReceipt;

const COORDINATION_APPLY_REPORT_SCHEMA: &str = crate::preserves_rail::COORDINATION_APPLY_REPORT_SCHEMA;
const COORDINATION_FENCING_TOKEN_SCHEMA: &str = crate::preserves_rail::COORDINATION_FENCING_TOKEN_SCHEMA;
const COORDINATION_RECEIPT_SCHEMA: &str = crate::preserves_rail::COORDINATION_RECEIPT_SCHEMA;
const COORDINATION_REQUEST_SCHEMA: &str = crate::preserves_rail::COORDINATION_REQUEST_SCHEMA;
const COORDINATION_SERVICE_MANIFEST_SCHEMA: &str = crate::preserves_rail::COORDINATION_SERVICE_MANIFEST_SCHEMA;
const COORDINATION_STATE_SNAPSHOT_SCHEMA: &str = crate::preserves_rail::COORDINATION_STATE_SNAPSHOT_SCHEMA;
const COORDINATION_STATUS_ASSERTION_SCHEMA: &str = crate::preserves_rail::COORDINATION_STATUS_ASSERTION_SCHEMA;
const COORDINATION_COMMIT_INTENT_SCHEMA: &str = "molten.coordination.commit-intent.v1";

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn content_ref_from_bytes(bytes: &[u8]) -> String {
    crate::preserves_rail::content_ref_from_bytes(bytes)
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

fn u64_value(value: u64) -> IoValue {
    crate::preserves_rail::u64_value(value)
}

fn validate_content_ref(value: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(value)
}

fn value_to_iovalue(value: &Value<IoValue>) -> IoValue {
    crate::preserves_rail::value_to_iovalue(value)
}

pub const SERVICE_LOCK: &str = "lock";
pub const SERVICE_QUEUE: &str = "queue";
pub const SERVICE_SEMAPHORE: &str = "semaphore";
pub const SERVICE_RATE_LIMIT: &str = "rate-limit";
pub const SERVICE_ELECTION: &str = "election";
pub const SERVICE_BARRIER: &str = "barrier";
pub const SERVICE_REGISTRY: &str = "registry";

pub const OP_ACQUIRE: &str = "acquire";
pub const OP_RELEASE: &str = "release";
pub const OP_ENQUEUE: &str = "enqueue";
pub const OP_DEQUEUE: &str = "dequeue";
pub const OP_ELECT: &str = "elect";
pub const OP_ARRIVE: &str = "arrive";
pub const OP_REGISTER: &str = "register";
pub const OP_UNREGISTER: &str = "unregister";
pub const OP_READ: &str = "read";
/// Releases a lock lease or leader term once the runtime's coordination clock reaches its expiry. It takes
/// no payload: requests never carry time.
pub const OP_EXPIRE: &str = "expire";
/// Steps the current leader down; only the leader holding the current fencing token may resign.
pub const OP_RESIGN: &str = "resign";
/// Requested from the coordination runtime, `linearizable` is denied: its in-process control registry has
/// no replicated read index.
pub const READ_CONSISTENCY_LINEARIZABLE: &str = crate::raft_control_plane::READ_CONSISTENCY_LINEARIZABLE;
/// Default coordination mode: committed state of the runtime's in-process control registry.
pub const READ_CONSISTENCY_LOCAL_COMMITTED: &str = crate::raft_control_plane::READ_CONSISTENCY_LOCAL_COMMITTED;
pub const READ_CONSISTENCY_LOCAL_STALE: &str = crate::raft_control_plane::READ_CONSISTENCY_LOCAL_STALE;

const TRANSITION_KIND_ADVANCE: &str = "advance";
const TRANSITION_KIND_DENY_PRESERVE: &str = "deny-preserve";
const TRANSITION_KIND_DUPLICATE_REPLAY: &str = "duplicate-replay";
const TRANSITION_KIND_CONFLICTING_DUPLICATE: &str = "conflicting-duplicate-deny";
const TRANSITION_KIND_READ_OBSERVE: &str = "read-observe";

const SHELL_INTENT_COMMIT: &str = "control-plane-commit";
const SHELL_INTENT_ASSERT_STATUS: &str = "dataspace-status-assertion";
const SHELL_INTENT_EMIT_RECEIPT: &str = "emit-coordination-receipt";
const SHELL_INTENT_REPLAY_OUTPUT: &str = "return-prior-output";

const COORDINATION_RECEIPT_FIELD_COUNT: usize = 13;
const COORDINATION_TRANSITION_FIELD_COUNT: usize = 8;

const COORDINATION_NAMESPACE_PREFIX: &str = "coordination";
pub const DEFAULT_COORDINATION_SERVICE_ID: &str = "coordination:local";
pub const DEFAULT_COORDINATION_QUEUE_CAPACITY: u64 = 4;
pub const DEFAULT_COORDINATION_SEMAPHORE_CAPACITY: u64 = 2;
pub const DEFAULT_COORDINATION_RATE_LIMIT: u64 = 2;
pub const DEFAULT_COORDINATION_BARRIER_PARTIES: u64 = 2;
/// Coordination time is sampled from the runtime's shell-owned [`CoordinationClock`] port, once per
/// admitted mutation, and recorded with each committed mutation as `(time_basis_ref, ticks)`. Lease lengths
/// are measured in the ticks of that basis (nanoseconds on the live node monotonic basis).
///
/// Lease length for a lock or leader term requested without `<lease ttl>`: 30 seconds on the live basis.
pub const DEFAULT_COORDINATION_LEASE_TICKS: u64 = 30_000_000_000;
/// Largest lease a lock or leader term may request (one hour on the live basis), so every holder eventually
/// becomes expirable.
pub const MAX_COORDINATION_LEASE_TICKS: u64 = 3_600_000_000_000;
/// Last clock sample a commit may record. Above it a maximal lease would not be representable, so further
/// mutations are denied instead of overflowing.
pub const MAX_COORDINATION_TIME_TICKS: u64 = u64::MAX - MAX_COORDINATION_LEASE_TICKS;

const MAX_COORDINATION_REFS: usize = 4096;
const MAX_COORDINATION_ITEMS: usize = 4096;
const MAX_COORDINATION_DIAGNOSTICS: usize = 256;
const MAX_COORDINATION_CHECKS: usize = 96;
const MAX_COORDINATION_SERVICES: usize = 16;
const MAX_COORDINATION_KEY_LEN: usize = 256;
const _: () = assert!(MAX_COORDINATION_REFS <= 100_000);
const _: () = assert!(MAX_COORDINATION_ITEMS <= 100_000);
const _: () = assert!(MAX_COORDINATION_DIAGNOSTICS <= 10_000);
const _: () = assert!(MAX_COORDINATION_CHECKS <= 10_000);
const _: () = assert!(MAX_COORDINATION_SERVICES <= 64);
const _: () = assert!(MAX_COORDINATION_KEY_LEN <= 4096);
const _: () = assert!(DEFAULT_COORDINATION_LEASE_TICKS > 0);
const _: () = assert!(DEFAULT_COORDINATION_LEASE_TICKS <= MAX_COORDINATION_LEASE_TICKS);
