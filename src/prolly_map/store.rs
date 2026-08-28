use molten_core::prolly_map::*;
use molten_node_host::node_state::NodeStateNamespace;
use molten_node_host::node_state::NodeStateNamespaceKind;
use molten_node_host::node_state::NodeStatePath;
use redb::ReadableDatabase;
use redb::ReadableTable;

use super::*;

const PROLLY_DATABASE_FILE: &str = "prolly-semantic-map.redb";
const PROLLY_BLOCKS_TABLE: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("prolly_blocks_v1");
const PROLLY_HEADS_TABLE: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("prolly_heads_v1");
const MAX_MAP_ID_BYTES: usize = 96;
const HEAD_FIELD_COUNT: usize = 7;
const HEAD_GENERATION_INDEX: usize = 0;
const HEAD_SCHEMA_INDEX: usize = 1;
const HEAD_PROFILE_INDEX: usize = 2;
const HEAD_TOP_NODE_INDEX: usize = 3;
const HEAD_HEIGHT_INDEX: usize = 4;
const HEAD_ENTRY_COUNT_INDEX: usize = 5;
const HEAD_ROOT_INDEX: usize = 6;
const MAX_HEAD_BYTES: usize = 1_024;

pub struct LocalProllyBlockStore {
    database: redb::Database,
}

impl LocalProllyBlockStore {
    pub fn open(storage: &NodeStateNamespace) -> ProllyPortResult<Self> {
        if storage.kind() != NodeStateNamespaceKind::Storage {
            return Err(port_error("prolly-storage-namespace", "Prolly store requires the storage namespace"));
        }
        let path = NodeStatePath::parse(PROLLY_DATABASE_FILE).map_err(molten_error)?;
        let file = storage.open_database_file(&path).map_err(molten_error)?;
        let database = redb::Database::builder().create_file(file).map_err(redb_error)?;
        initialize(&database)?;
        Ok(Self { database })
    }
}

impl ProllyBlockStorePort for LocalProllyBlockStore {
    fn read_block(&self, node_ref: &NodeRef) -> ProllyPortResult<Option<Vec<u8>>> {
        let read = self.database.begin_read().map_err(redb_error)?;
        let table = read.open_table(PROLLY_BLOCKS_TABLE).map_err(redb_error)?;
        table
            .get(node_ref.as_str())
            .map_err(redb_error)
            .map(|value| value.map(|guard| guard.value().to_vec()))
    }

    fn stage_blocks(&mut self, blocks: &[EncodedBlock]) -> ProllyPortResult<()> {
        let write = self.database.begin_write().map_err(redb_error)?;
        {
            let mut table = write.open_table(PROLLY_BLOCKS_TABLE).map_err(redb_error)?;
            for block in blocks {
                if let Some(existing) = table.get(block.node_ref.as_str()).map_err(redb_error)?
                    && existing.value() != block.bytes.as_slice()
                {
                    return Err(port_error("prolly-block-collision", "existing block bytes do not match identity"));
                }
                table.insert(block.node_ref.as_str(), block.bytes.as_slice()).map_err(redb_error)?;
            }
        }
        write.commit().map_err(redb_error)
    }

    fn read_root(&self, map_id: &str) -> ProllyPortResult<Option<PublishedProllyRoot>> {
        validate_map_id(map_id)?;
        let read = self.database.begin_read().map_err(redb_error)?;
        let table = read.open_table(PROLLY_HEADS_TABLE).map_err(redb_error)?;
        table.get(map_id).map_err(redb_error)?.map(|guard| decode_head(guard.value())).transpose()
    }

    fn compare_and_advance(
        &mut self,
        map_id: &str,
        expected: &ExpectedProllyRoot,
        next: &PublishedProllyRoot,
    ) -> ProllyPortResult<ProllyPublicationObservation> {
        validate_map_id(map_id)?;
        let write = self.database.begin_write().map_err(redb_error)?;
        let observed = {
            let table = write.open_table(PROLLY_HEADS_TABLE).map_err(redb_error)?;
            table.get(map_id).map_err(redb_error)?.map(|guard| decode_head(guard.value())).transpose()?
        };
        if observed.as_ref() == Some(next) {
            return Ok(ProllyPublicationObservation::AlreadyApplied);
        }
        if !expected_matches(expected, observed.as_ref()) {
            return Ok(ProllyPublicationObservation::Stale);
        }
        let bytes = encode_head(next)?;
        {
            let mut table = write.open_table(PROLLY_HEADS_TABLE).map_err(redb_error)?;
            table.insert(map_id, bytes.as_slice()).map_err(redb_error)?;
        }
        write.commit().map_err(redb_error)?;
        Ok(ProllyPublicationObservation::Applied)
    }

    fn delete_blocks(&mut self, node_refs: &[NodeRef]) -> ProllyPortResult<()> {
        let write = self.database.begin_write().map_err(redb_error)?;
        {
            let mut table = write.open_table(PROLLY_BLOCKS_TABLE).map_err(redb_error)?;
            for node_ref in node_refs {
                table.remove(node_ref.as_str()).map_err(redb_error)?;
            }
        }
        write.commit().map_err(redb_error)
    }
}

fn initialize(database: &redb::Database) -> ProllyPortResult<()> {
    let write = database.begin_write().map_err(redb_error)?;
    {
        let _blocks = write.open_table(PROLLY_BLOCKS_TABLE).map_err(redb_error)?;
        let _heads = write.open_table(PROLLY_HEADS_TABLE).map_err(redb_error)?;
    }
    write.commit().map_err(redb_error)
}

fn encode_head(head: &PublishedProllyRoot) -> ProllyPortResult<Vec<u8>> {
    let text = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n",
        head.generation,
        head.root.schema,
        head.root.profile_ref.as_str(),
        head.root.top_node_ref.as_str(),
        head.root.height,
        head.root.entry_count,
        head.root.root_ref.as_str()
    );
    if text.len() > MAX_HEAD_BYTES {
        return Err(port_error("prolly-head-bound", "encoded head exceeds its bound"));
    }
    Ok(text.into_bytes())
}

fn decode_head(bytes: &[u8]) -> ProllyPortResult<PublishedProllyRoot> {
    if bytes.len() > MAX_HEAD_BYTES {
        return Err(port_error("prolly-head-bound", "stored head exceeds its bound"));
    }
    let text = std::str::from_utf8(bytes).map_err(|error| {
        ProllyPortError::new("prolly-head-utf8", format!("stored head is not UTF-8: {error}"), false)
    })?;
    let fields = text.lines().collect::<Vec<_>>();
    if fields.len() != HEAD_FIELD_COUNT {
        return Err(port_error("prolly-head-shape", "stored head has the wrong field count"));
    }
    let generation = fields[HEAD_GENERATION_INDEX]
        .parse::<u64>()
        .map_err(|error| ProllyPortError::new("prolly-head-generation", error.to_string(), false))?;
    let height = fields[HEAD_HEIGHT_INDEX]
        .parse::<u16>()
        .map_err(|error| ProllyPortError::new("prolly-head-height", error.to_string(), false))?;
    let entry_count = fields[HEAD_ENTRY_COUNT_INDEX]
        .parse::<u32>()
        .map_err(|error| ProllyPortError::new("prolly-head-count", error.to_string(), false))?;
    Ok(PublishedProllyRoot {
        root: ProllyRoot {
            schema: fields[HEAD_SCHEMA_INDEX].to_string(),
            profile_ref: ProfileRef::new(fields[HEAD_PROFILE_INDEX].to_string()),
            top_node_ref: NodeRef::new(fields[HEAD_TOP_NODE_INDEX].to_string()),
            height,
            entry_count,
            root_ref: RootRef::new(fields[HEAD_ROOT_INDEX].to_string()),
        },
        generation,
    })
}

fn expected_matches(expected: &ExpectedProllyRoot, observed: Option<&PublishedProllyRoot>) -> bool {
    match (expected.root_ref.as_ref(), observed) {
        (None, None) => expected.generation == 0,
        (Some(expected_ref), Some(observed)) => {
            expected.generation == observed.generation && expected_ref == &observed.root.root_ref
        }
        _ => false,
    }
}

fn validate_map_id(value: &str) -> ProllyPortResult<()> {
    if value.is_empty()
        || value.len() > MAX_MAP_ID_BYTES
        || !value.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(port_error("prolly-map-id", "map id is invalid"));
    }
    Ok(())
}

fn redb_error(error: impl std::fmt::Display) -> ProllyPortError {
    ProllyPortError::new("prolly-redb", error.to_string(), false)
}

fn molten_error(error: impl std::fmt::Display) -> ProllyPortError {
    ProllyPortError::new("prolly-node-state", error.to_string(), false)
}

fn port_error(code: &'static str, detail: &'static str) -> ProllyPortError {
    ProllyPortError::new(code, detail, false)
}
