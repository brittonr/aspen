use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use molten_core::world_commit::RootKind;
use molten_core::world_commit::WorldCommitRef;
use molten_core::world_commit::WorldRootRef;

#[derive(Default)]
struct Memory {
    objects: BTreeMap<WorldRootRef, Vec<u8>>,
    commits: BTreeMap<WorldCommitRef, Vec<WorldRootRef>>,
    heads: Vec<WorldCommitRef>,
    parents: Vec<WorldCommitRef>,
    events: Vec<&'static str>,
    handler_calls: usize,
}

struct MemoryPort(Rc<RefCell<Memory>>);

fn identity(bytes: &[u8]) -> String {
    codec::content_ref_from_bytes(bytes)
}
fn schema_ref(label: &str) -> WorldMergeSchemaRef {
    WorldMergeSchemaRef::new(identity(label.as_bytes())).expect("schema")
}
fn head(label: &str) -> WorldCommitRef {
    WorldCommitRef::new(identity(label.as_bytes())).expect("head")
}
fn stored_value(memory: &mut Memory, schema: &str, bytes: &[u8]) -> WorldMergeValue {
    let root = WorldRootRef::parse(RootKind::DurableState, identity(bytes)).expect("root");
    memory.objects.insert(root.clone(), bytes.to_vec());
    WorldMergeValue {
        root: Some(root),
        schema_ref: Some(schema_ref(schema)),
        available: true,
        canonical_bytes: None,
        keyed_values: BTreeMap::new(),
    }
}

fn fixture() -> (WorldMergeRequest, Rc<RefCell<Memory>>) {
    let mut memory = Memory::default();
    let target = stored_value(&mut memory, "v2", b"modern-base");
    let source = stored_value(&mut memory, "v1", b"legacy-left");
    memory.heads = vec![head("left"), head("right")];
    let request = WorldMergeRequest {
        base_head: head("base"),
        source_heads: memory.heads.clone(),
        common_ancestor_verified: true,
        common_ancestor_ambiguous: false,
        roots: vec![WorldMergeRootInput {
            kind: RootKind::DurableState,
            base: target.clone(),
            left: source,
            right: target,
        }],
        profile: WorldMergeProfile {
            profile_ref: WorldMergeProfileRef::new(identity(b"profile")).expect("profile"),
            policy_ref: WorldMergePolicyRef::new(identity(b"policy")).expect("policy"),
            root_modes: BTreeMap::from([(RootKind::DurableState, WorldMergeMode::AncestorReplacement)]),
            migrations: BTreeMap::from([(RootKind::DurableState, WorldMigrationBinding {
                plan_ref: WorldMigrationPlanRef::new(identity(b"migration")).expect("migration"),
                profile_id: "v1-to-v2".to_string(),
                source_schema: schema_ref("v1"),
                target_schema: schema_ref("v2"),
                admitted: true,
            })]),
            handlers: BTreeMap::new(),
        },
        bounds: WorldMergeBounds::standard(),
    };
    (request, Rc::new(RefCell::new(memory)))
}

impl WorldMergeObjectPort for MemoryPort {
    fn load_root(&mut self, root: &WorldRootRef, _: u64) -> Result<Vec<u8>, WorldMergePortError> {
        self.0
            .borrow()
            .objects
            .get(root)
            .cloned()
            .ok_or_else(|| WorldMergePortError::new("load", "missing object"))
    }
    fn persist_generated_root(
        &mut self,
        kind: RootKind,
        _: &WorldMergeSchemaRef,
        bytes: &[u8],
    ) -> Result<WorldRootRef, WorldMergePortError> {
        let root = WorldRootRef::parse(kind, identity(bytes)).expect("persisted identity");
        let mut memory = self.0.borrow_mut();
        memory.objects.insert(root.clone(), bytes.to_vec());
        memory.events.push("persist");
        Ok(root)
    }
}
impl WorldMergeCommitPort for MemoryPort {
    fn publish_merge_commit(
        &mut self,
        base: &WorldCommitRef,
        sources: &[WorldCommitRef],
        roots: &[WorldRootRef],
    ) -> Result<WorldCommitRef, WorldMergePortError> {
        let mut memory = self.0.borrow_mut();
        for root in roots {
            assert!(memory.objects.contains_key(root), "commit preceded root persistence");
        }
        memory.parents = std::iter::once(base.clone()).chain(sources.iter().cloned()).collect();
        let commit = head("merged");
        memory.commits.insert(commit.clone(), roots.to_vec());
        memory.heads = vec![commit.clone()];
        memory.events.push("commit");
        Ok(commit)
    }
}
impl WorldMergeConflictPort for MemoryPort {
    fn persist_conflict(&mut self, _: &str, _: &[u8]) -> Result<(), WorldMergePortError> {
        panic!("migration scenario must not publish conflict artifacts")
    }
}
impl WorldMergeAuthorityPort for MemoryPort {
    fn recheck_merge_authority(
        &mut self,
        _: &[WorldCommitRef],
        _: &WorldMergePolicyRef,
    ) -> Result<String, WorldMergePortError> {
        self.0.borrow_mut().events.push("authority");
        Ok(identity(b"authority"))
    }
}
impl WorldMergeHandlerPort for MemoryPort {
    fn load_handler(
        &mut self,
        _: &WorldApplicationHandlerProfile,
    ) -> Result<Box<dyn WorldMergeHandler>, WorldMergePortError> {
        self.0.borrow_mut().handler_calls += 1;
        Err(WorldMergePortError::new("handler", "unexpected handler execution"))
    }
}

struct MigrationAdapter {
    case: &'static str,
    calls: usize,
}
impl WorldMergeMigrationPort for MigrationAdapter {
    fn materialize_migration(
        &mut self,
        conversion: &AdmittedWorldMigration,
        source: &[u8],
    ) -> Result<WorldMigrationResult, WorldMergePortError> {
        self.calls += 1;
        assert_eq!(source, b"legacy-left", "adapter must receive original source bytes");
        if self.case == "adapter-failure" {
            return Err(WorldMergePortError::new("adapter", "injected failure"));
        }
        let mut result = WorldMigrationResult::new(conversion, b"modern-left".to_vec(), BTreeMap::new())
            .map_err(|issue| WorldMergePortError::new("result", format!("{issue:?}")))?;
        match self.case {
            "empty" => result.canonical_bytes.clear(),
            "oversized" => {
                result.canonical_bytes = vec![0; usize::try_from(conversion.max_output_bytes()).expect("bound") + 1]
            }
            "wrong-target" => result.target_schema = schema_ref("wrong-target"),
            "substituted-content" => result.canonical_bytes = b"substitution".to_vec(),
            "stale-conversion" => result.conversion_ref = identity(b"stale"),
            "keyed-substitution" => {
                result.keyed_values.insert("injected".to_string(), b"value".to_vec());
            }
            "positive" => {}
            _ => panic!("unknown adapter case"),
        }
        Ok(result)
    }
}

fn prepare_and_publish(
    request: &WorldMergeRequest,
    memory: &Rc<RefCell<Memory>>,
    adapter: &mut MigrationAdapter,
) -> bool {
    let prepared =
        prepare_world_merge_plan(&mut MemoryPort(memory.clone()), adapter, &mut MemoryPort(memory.clone()), request);
    if let Ok(plan) = prepared {
        publish_world_merge(
            &mut MemoryPort(memory.clone()),
            &mut MemoryPort(memory.clone()),
            &mut MemoryPort(memory.clone()),
            &mut MemoryPort(memory.clone()),
            &WorldMergePublicationRequest {
                plan: &plan,
                policy_ref: &request.profile.policy_ref,
            },
        )
        .expect("publish prepared plan");
        true
    } else {
        false
    }
}
