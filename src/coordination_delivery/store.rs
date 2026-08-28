use molten_core::coordination_delivery::*;
use molten_node_host::node_state::NodeStateNamespace;
use molten_node_host::node_state::NodeStateNamespaceKind;
use molten_node_host::node_state::NodeStatePath;
use redb::ReadableDatabase;
use redb::ReadableTable;

use super::*;

const DELIVERY_DATABASE_FILE: &str = "coordination-delivery.redb";
const DELIVERY_STATE_TABLE: redb::TableDefinition<&str, &[u8]> =
    redb::TableDefinition::new("coordination_delivery_states_v1");
const MAX_DELIVERY_STATE_BYTES: usize = 4_194_304;
const MAX_QUEUE_ID_BYTES: usize = 192;

pub struct LocalDeliveryStore {
    database: redb::Database,
    engine_epoch: u64,
}

impl LocalDeliveryStore {
    pub fn open(storage: &NodeStateNamespace, engine_epoch: u64) -> DeliveryPortResult<Self> {
        if storage.kind() != NodeStateNamespaceKind::Storage {
            return Err(port_error(
                "delivery-storage-namespace",
                "coordination delivery store requires the storage namespace",
            ));
        }
        if engine_epoch == 0 {
            return Err(port_error("delivery-engine-epoch", "coordination delivery engine epoch must be positive"));
        }
        let path = NodeStatePath::parse(DELIVERY_DATABASE_FILE).map_err(node_state_error)?;
        let file = storage.open_database_file(&path).map_err(node_state_error)?;
        let database = redb::Database::builder().create_file(file).map_err(redb_error)?;
        initialize(&database)?;
        Ok(Self { database, engine_epoch })
    }
}

impl DeliveryCommitPort for LocalDeliveryStore {
    fn load(&self, queue_id: &str) -> DeliveryPortResult<Option<PublishedDeliveryState>> {
        validate_queue_id(queue_id)?;
        let read = self.database.begin_read().map_err(redb_error)?;
        let table = read.open_table(DELIVERY_STATE_TABLE).map_err(redb_error)?;
        table
            .get(queue_id)
            .map_err(redb_error)?
            .map(|guard| decode_published_state(guard.value()))
            .transpose()
    }

    fn compare_and_commit(&mut self, request: &DeliveryCommitRequest) -> DeliveryPortResult<DeliveryCommitObservation> {
        validate_queue_id(&request.queue_id)?;
        validate_published_state(&request.next)?;
        let write = self.database.begin_write().map_err(redb_error)?;
        let observed = {
            let table = write.open_table(DELIVERY_STATE_TABLE).map_err(redb_error)?;
            table
                .get(request.queue_id.as_str())
                .map_err(redb_error)?
                .map(|guard| decode_published_state(guard.value()))
                .transpose()?
        };
        if observed.as_ref() == Some(&request.next) {
            return Ok(observation(
                DeliveryCommitDisposition::AlreadyApplied,
                self.engine_epoch,
                Some(request.next.state_ref.clone()),
            ));
        }
        if request.requested_engine_epoch != self.engine_epoch
            || !expected_matches(&request.expected, observed.as_ref())
        {
            return Ok(DeliveryCommitObservation {
                disposition: DeliveryCommitDisposition::Stale,
                currentness: DeliveryCurrentness::Linearizable,
                durability: DeliveryDurabilityOutcome::Durable,
                engine_epoch: self.engine_epoch,
                observed_state_ref: observed.map(|published| published.state_ref),
            });
        }
        let bytes = encode_published_state(&request.next)?;
        {
            let mut table = write.open_table(DELIVERY_STATE_TABLE).map_err(redb_error)?;
            table.insert(request.queue_id.as_str(), bytes.as_slice()).map_err(redb_error)?;
        }
        write.commit().map_err(redb_error)?;
        Ok(observation(
            DeliveryCommitDisposition::Applied,
            self.engine_epoch,
            Some(request.next.state_ref.clone()),
        ))
    }
}

fn initialize(database: &redb::Database) -> DeliveryPortResult<()> {
    let write = database.begin_write().map_err(redb_error)?;
    {
        let _table = write.open_table(DELIVERY_STATE_TABLE).map_err(redb_error)?;
    }
    write.commit().map_err(redb_error)
}

fn encode_published_state(published: &PublishedDeliveryState) -> DeliveryPortResult<Vec<u8>> {
    validate_published_state(published)?;
    let bytes = serde_json::to_vec(&published.state).map_err(codec_error)?;
    if bytes.len() > MAX_DELIVERY_STATE_BYTES {
        return Err(port_error("delivery-state-bound", "coordination delivery state exceeds its byte bound"));
    }
    Ok(bytes)
}

fn decode_published_state(bytes: &[u8]) -> DeliveryPortResult<PublishedDeliveryState> {
    if bytes.len() > MAX_DELIVERY_STATE_BYTES {
        return Err(port_error("delivery-state-bound", "stored coordination delivery state exceeds its byte bound"));
    }
    let state = serde_json::from_slice::<DeliveryState>(bytes).map_err(codec_error)?;
    Ok(PublishedDeliveryState::from_state(state))
}

fn validate_published_state(published: &PublishedDeliveryState) -> DeliveryPortResult<()> {
    if published.state_ref != identify_delivery_state(&published.state)
        || published.revision != published.state.revision
    {
        return Err(port_error(
            "delivery-state-identity",
            "coordination delivery state identity or revision does not match",
        ));
    }
    Ok(())
}

fn expected_matches(expected: &ExpectedDeliveryState, observed: Option<&PublishedDeliveryState>) -> bool {
    match (&expected.state_ref, observed) {
        (None, None) => expected.revision == INITIAL_DELIVERY_REVISION,
        (Some(expected_ref), Some(observed)) => {
            expected.revision == observed.revision && expected_ref == &observed.state_ref
        }
        _ => false,
    }
}

fn observation(
    disposition: DeliveryCommitDisposition,
    engine_epoch: u64,
    observed_state_ref: Option<String>,
) -> DeliveryCommitObservation {
    DeliveryCommitObservation {
        disposition,
        currentness: DeliveryCurrentness::Linearizable,
        durability: DeliveryDurabilityOutcome::Durable,
        engine_epoch,
        observed_state_ref,
    }
}

fn validate_queue_id(value: &str) -> DeliveryPortResult<()> {
    if value.is_empty()
        || value.len() > MAX_QUEUE_ID_BYTES
        || !value.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(port_error("delivery-queue-id", "coordination delivery queue id is invalid"));
    }
    Ok(())
}

fn redb_error(error: impl std::fmt::Display) -> DeliveryPortError {
    DeliveryPortError::new("delivery-redb", error.to_string(), false)
}

fn codec_error(error: impl std::fmt::Display) -> DeliveryPortError {
    DeliveryPortError::new("delivery-state-codec", error.to_string(), false)
}

fn node_state_error(error: impl std::fmt::Display) -> DeliveryPortError {
    DeliveryPortError::new("delivery-node-state", error.to_string(), false)
}

fn port_error(code: &'static str, detail: &'static str) -> DeliveryPortError {
    DeliveryPortError::new(code, detail, false)
}
