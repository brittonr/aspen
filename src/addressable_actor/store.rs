use molten_core::addressable_actor::*;
use redb::ReadableDatabase;
use redb::ReadableTable;

use super::*;

const ACTOR_DATABASE_FILE: &str = "addressable-actor.redb";
const ACTOR_STATE_TABLE: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("addressable_actor_states_v1");
const MAX_ACTOR_STATE_BYTES: usize = 2_097_152;

pub struct Local {
    database: redb::Database,
    engine_epoch: u64,
}

impl Local {
    pub fn open(storage: &molten_node_host::node_state::Scope, engine_epoch: u64) -> PortResult<Self> {
        if storage.kind() != molten_node_host::node_state::NamespaceKind::Storage {
            return Err(port_error(
                "actor-storage-namespace",
                "addressable actor store requires the storage namespace",
            ));
        }
        if engine_epoch == 0 {
            return Err(port_error("actor-engine-epoch", "addressable actor engine epoch must be positive"));
        }
        let path = molten_node_host::node_state::RelativePath::parse(ACTOR_DATABASE_FILE).map_err(node_state_error)?;
        let file = storage.open_database_file(&path).map_err(node_state_error)?;
        let database = redb::Database::builder().create_file(file).map_err(redb_error)?;
        initialize(&database)?;
        Ok(Self { database, engine_epoch })
    }
}

impl ActorCommitPort for Local {
    fn load(&self, actor_key_ref: &str) -> PortResult<Option<PublishedState>> {
        validate_key_ref(actor_key_ref)?;
        let read = self.database.begin_read().map_err(redb_error)?;
        let table = read.open_table(ACTOR_STATE_TABLE).map_err(redb_error)?;
        table
            .get(actor_key_ref)
            .map_err(redb_error)?
            .map(|guard| decode_published_state(guard.value()))
            .transpose()
    }

    fn compare_and_commit(&mut self, request: &CommitRequest) -> PortResult<CommitObservation> {
        validate_key_ref(&request.actor_key_ref)?;
        validate_published_state(&request.next)?;
        let write = self.database.begin_write().map_err(redb_error)?;
        let observed = {
            let table = write.open_table(ACTOR_STATE_TABLE).map_err(redb_error)?;
            table
                .get(request.actor_key_ref.as_str())
                .map_err(redb_error)?
                .map(|guard| decode_published_state(guard.value()))
                .transpose()?
        };
        if observed.as_ref() == Some(&request.next) {
            return Ok(observation(
                CommitDisposition::AlreadyApplied,
                self.engine_epoch,
                Some(request.next.state_ref.clone()),
            ));
        }
        if request.requested_engine_epoch != self.engine_epoch
            || !expected_matches(&request.expected, observed.as_ref())
        {
            return Ok(CommitObservation {
                disposition: CommitDisposition::Stale,
                currentness: CommitCurrentness::Linearizable,
                durability: DurabilityOutcome::Durable,
                engine_epoch: self.engine_epoch,
                observed_state_ref: observed.map(|published| published.state_ref),
            });
        }
        let bytes = encode_published_state(&request.next)?;
        {
            let mut table = write.open_table(ACTOR_STATE_TABLE).map_err(redb_error)?;
            table.insert(request.actor_key_ref.as_str(), bytes.as_slice()).map_err(redb_error)?;
        }
        write.commit().map_err(redb_error)?;
        Ok(observation(CommitDisposition::Applied, self.engine_epoch, Some(request.next.state_ref.clone())))
    }
}

fn initialize(database: &redb::Database) -> PortResult<()> {
    let write = database.begin_write().map_err(redb_error)?;
    {
        let _table = write.open_table(ACTOR_STATE_TABLE).map_err(redb_error)?;
    }
    write.commit().map_err(redb_error)
}

fn encode_published_state(published: &PublishedState) -> PortResult<Vec<u8>> {
    validate_published_state(published)?;
    let bytes = serde_json::to_vec(&published.state).map_err(codec_error)?;
    if bytes.len() > MAX_ACTOR_STATE_BYTES {
        return Err(port_error("actor-state-bound", "addressable actor state exceeds its byte bound"));
    }
    Ok(bytes)
}

fn decode_published_state(bytes: &[u8]) -> PortResult<PublishedState> {
    if bytes.len() > MAX_ACTOR_STATE_BYTES {
        return Err(port_error("actor-state-bound", "stored addressable actor state exceeds its byte bound"));
    }
    let state = serde_json::from_slice::<ActorState>(bytes).map_err(codec_error)?;
    if !validate_actor_state(&state).is_empty() {
        return Err(port_error("actor-state-validation", "stored addressable actor state is invalid"));
    }
    Ok(PublishedState::from_state(state))
}

fn validate_published_state(published: &PublishedState) -> PortResult<()> {
    if !validate_actor_state(&published.state).is_empty()
        || published.state_ref != identify_actor_state(&published.state)
        || published.revision != published.state.revision
        || published.state.actor_key_ref.is_empty()
    {
        return Err(port_error("actor-state-identity", "addressable actor state identity or revision does not match"));
    }
    Ok(())
}

fn expected_matches(expected: &ExpectedState, observed: Option<&PublishedState>) -> bool {
    match (&expected.state_ref, observed) {
        (None, None) => expected.revision == ADDRESSABLE_ACTOR_INITIAL_REVISION,
        (Some(expected_ref), Some(observed)) => {
            expected.revision == observed.revision && expected_ref == &observed.state_ref
        }
        _ => false,
    }
}

fn observation(
    disposition: CommitDisposition,
    engine_epoch: u64,
    observed_state_ref: Option<String>,
) -> CommitObservation {
    CommitObservation {
        disposition,
        currentness: CommitCurrentness::Linearizable,
        durability: DurabilityOutcome::Durable,
        engine_epoch,
        observed_state_ref,
    }
}

fn validate_key_ref(value: &str) -> PortResult<()> {
    if !valid_reference(value) {
        return Err(port_error("actor-key-ref", "addressable actor key reference is invalid"));
    }
    Ok(())
}

fn redb_error(error: impl std::fmt::Display) -> PortError {
    PortError::new("actor-redb", error.to_string(), false)
}

fn codec_error(error: impl std::fmt::Display) -> PortError {
    PortError::new("actor-state-codec", error.to_string(), false)
}

fn node_state_error(error: impl std::fmt::Display) -> PortError {
    PortError::new("actor-node-state", error.to_string(), false)
}

fn port_error(code: &'static str, detail: &'static str) -> PortError {
    PortError::new(code, detail, false)
}
