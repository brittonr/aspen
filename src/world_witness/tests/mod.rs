mod consumer;
mod external;

/// Genuine independent provider persistence and complete consumer DB restoration.
/// Preexisting local records are fixtures: this does not claim authorized shell advance.
// r[verify molten.world_witness.verification]
#[test]
fn whole_consumer_database_rollback_is_detected_by_fresh_authenticated_query() {
    let consumer = consumer::Fixture::new("world-witness-consumer-rollback");
    let external_workspace = crate::test_support::Workspace::new("world-witness-independent-index").unwrap();
    let external_state = external_workspace.state().unwrap();
    let external_plan = external_workspace.process_bridge().plan(&external_state).unwrap();
    let external_path = external_plan.path().join("independent-index");
    let mut provider = external::initialize(&external_path);
    let first = external::claim(1, None);
    let first_revision = external::commit(&mut provider, first.clone());
    consumer.seed(&first, first_revision);
    provider.observe_current(&first, first_revision).unwrap();
    let snapshot = std::fs::read(consumer.path()).unwrap();
    let retained_before = consumer.retained();

    let second = external::claim(2, Some(first.digest));
    let second_revision = external::commit(&mut provider, second.clone());
    consumer.seed(&second, second_revision);
    assert_eq!(consumer.retained().subject.generation, 2);
    provider.observe_current(&second, second_revision).unwrap();
    drop(provider);

    // No open consumer database remains. Restore the entire file, including
    // heads, generations, witness highwater, transitions and staged tables.
    std::fs::write(consumer.path(), &snapshot).unwrap();
    let restored = consumer.retained();
    assert_eq!(restored, retained_before);
    let mut heads = consumer.open();
    {
        let store = crate::world_witness::Database::new(&mut heads).unwrap();
        assert_eq!(store.current(&consumer::branch()).unwrap().unwrap().generation, 1);
    }
    drop(heads);

    // Reopen the SAME independent provider DB: its newer state was not restored.
    let mut provider = external::reopen(&external_path);
    let query = provider.query(restored.checkpoint.revision).unwrap();
    assert_eq!(query.snapshot.revision, second_revision);
    assert_eq!(query.snapshot.committed, Some(second.clone()));
    assert_eq!(
        provider.observe_current(&first, restored.checkpoint.revision).unwrap_err().kind,
        super::provider::ErrorKind::Invalid
    );
    provider.observe_current(&second, second_revision).unwrap();
}

// r[verify molten.world_witness.verification]
#[test]
fn missing_consumer_highest_is_not_reenrollment_when_head_survives() {
    let consumer = consumer::Fixture::new("world-witness-missing-highest");
    consumer.seed(&external::claim(1, None), 3);
    let mut heads = consumer.open();
    {
        let write = heads.database().begin_write().unwrap();
        write.open_table(super::store::HIGHEST).unwrap().remove(consumer::branch().as_str()).unwrap();
        write.commit().unwrap();
    }
    let store = crate::world_witness::Database::new(&mut heads).unwrap();
    assert_eq!(
        store.retained(&consumer::branch()).unwrap(),
        molten_core::world_witness::RetainedWitnessState::Missing
    );
    assert_eq!(store.current(&consumer::branch()).unwrap().unwrap().generation, 1);
}
