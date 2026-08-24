use super::*;

const CAPACITY: u32 = 2;
const HIGH_WATERMARK: u32 = 2;
const LOW_WATERMARK: u32 = 1;
const STRICT_LRU_THRESHOLD: u8 = 0;
const FIFO_THRESHOLD: u8 = 100;
const FIRST_VALUE: u32 = 11;
const SECOND_VALUE: u32 = 22;
const THIRD_VALUE: u32 = 33;

fn policy(threshold: u8) -> molten_core::fabric_durability::cache::Policy {
    molten_core::fabric_durability::cache::Policy::new(CAPACITY, HIGH_WATERMARK, LOW_WATERMARK, threshold)
        .expect("valid store policy")
}

fn arguments(subject: &str) -> Vec<String> {
    vec![format!("subject={subject}"), "operation=observe".to_string()]
}

fn key(subject: &str) -> String {
    let arguments = arguments(subject);
    molten_core::fabric_durability::cache::project_key(&molten_core::fabric_durability::cache::AccessProjection {
        dataspace_identity: "dataspace:main",
        normalized_arguments: &arguments,
        capability_context: Some("capability:read"),
    })
    .expect("projected key")
}

fn lookup(
    store: &Store<u32>,
    subject: &str,
    value: u32,
    loads: &std::sync::atomic::AtomicU32,
) -> Result<Access<u32>, LookupError<&'static str>> {
    let arguments = arguments(subject);
    store.lookup_or_load(
        &molten_core::fabric_durability::cache::AccessProjection {
            dataspace_identity: "dataspace:main",
            normalized_arguments: &arguments,
            capability_context: Some("capability:read"),
        },
        || {
            loads.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(value)
        },
    )
}

// r[verify aspen.dataspace_access_cache.bound]
// r[verify aspen.dataspace_access_cache.boundary]
// r[verify aspen.dataspace_access_cache.verification]
#[test]
fn miss_uses_loader_once_and_hit_reuses_the_bounded_value() {
    let store = Store::new(policy(STRICT_LRU_THRESHOLD));
    let loads = std::sync::atomic::AtomicU32::new(0);

    let first = lookup(&store, "alpha", FIRST_VALUE, &loads).expect("first access");
    let second = lookup(&store, "alpha", SECOND_VALUE, &loads).expect("second access");

    assert_eq!(first.source, Source::Loaded);
    assert_eq!(second.source, Source::Hit);
    assert_eq!(*first.value, FIRST_VALUE);
    assert_eq!(*second.value, FIRST_VALUE);
    assert_eq!(loads.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(store.len(), Ok(1));
}

#[test]
fn adapter_error_is_returned_without_an_entry() {
    let store = Store::<u32>::new(policy(STRICT_LRU_THRESHOLD));
    let arguments = arguments("denied");
    let result = store.lookup_or_load(
        &molten_core::fabric_durability::cache::AccessProjection {
            dataspace_identity: "dataspace:main",
            normalized_arguments: &arguments,
            capability_context: Some("capability:read"),
        },
        || Err("adapter-denied"),
    );

    assert!(matches!(result, Err(LookupError::Load("adapter-denied"))));
    assert_eq!(store.len(), Ok(0));
}

// r[verify aspen.dataspace_access_cache.decision]
// r[verify aspen.dataspace_access_cache.verification]
#[test]
fn strict_lru_promotes_hits_before_eviction() {
    let store = Store::new(policy(STRICT_LRU_THRESHOLD));
    let loads = std::sync::atomic::AtomicU32::new(0);
    drop(lookup(&store, "alpha", FIRST_VALUE, &loads).expect("alpha"));
    drop(lookup(&store, "beta", SECOND_VALUE, &loads).expect("beta"));
    drop(lookup(&store, "alpha", FIRST_VALUE, &loads).expect("alpha hit"));
    drop(lookup(&store, "gamma", THIRD_VALUE, &loads).expect("gamma"));

    let state = store.state.lock().expect("state");
    assert!(state.entries.contains_key(&key("alpha")));
    assert!(!state.entries.contains_key(&key("beta")));
    assert!(state.entries.contains_key(&key("gamma")));
    assert!(state.entries.len() <= usize::try_from(CAPACITY).expect("capacity fits usize"));
}

#[test]
fn fifo_does_not_promote_hits_before_eviction() {
    let store = Store::new(policy(FIFO_THRESHOLD));
    let loads = std::sync::atomic::AtomicU32::new(0);
    drop(lookup(&store, "alpha", FIRST_VALUE, &loads).expect("alpha"));
    drop(lookup(&store, "beta", SECOND_VALUE, &loads).expect("beta"));
    drop(lookup(&store, "alpha", FIRST_VALUE, &loads).expect("alpha hit"));
    drop(lookup(&store, "gamma", THIRD_VALUE, &loads).expect("gamma"));

    let state = store.state.lock().expect("state");
    assert!(!state.entries.contains_key(&key("alpha")));
    assert!(state.entries.contains_key(&key("beta")));
    assert!(state.entries.contains_key(&key("gamma")));
}

struct DropProbe {
    store: std::sync::Weak<Store<DropProbe>>,
    observed_unlocked_store: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Drop for DropProbe {
    fn drop(&mut self) {
        if let Some(store) = self.store.upgrade() {
            self.observed_unlocked_store
                .fetch_or(store.try_len().is_some(), std::sync::atomic::Ordering::SeqCst);
        }
    }
}

// r[verify aspen.dataspace_access_cache.deferral]
// r[verify aspen.dataspace_access_cache.verification]
#[test]
fn evicted_value_drops_after_the_guard_is_released() {
    const SINGLE_CAPACITY: u32 = 1;
    const SINGLE_HIGH: u32 = 1;
    const SINGLE_LOW: u32 = 0;
    let single =
        molten_core::fabric_durability::cache::Policy::new(SINGLE_CAPACITY, SINGLE_HIGH, SINGLE_LOW, FIFO_THRESHOLD)
            .expect("single policy");
    let store = std::sync::Arc::new(Store::new(single));
    let observed_unlocked_store = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let loads = std::sync::atomic::AtomicU32::new(0);

    let first_arguments = arguments("first");
    let first = store
        .lookup_or_load(
            &molten_core::fabric_durability::cache::AccessProjection {
                dataspace_identity: "dataspace:main",
                normalized_arguments: &first_arguments,
                capability_context: None,
            },
            || {
                loads.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok::<_, &'static str>(DropProbe {
                    store: std::sync::Arc::downgrade(&store),
                    observed_unlocked_store: std::sync::Arc::clone(&observed_unlocked_store),
                })
            },
        )
        .expect("first probe");
    drop(first);

    let second_arguments = arguments("second");
    let second = store
        .lookup_or_load(
            &molten_core::fabric_durability::cache::AccessProjection {
                dataspace_identity: "dataspace:main",
                normalized_arguments: &second_arguments,
                capability_context: None,
            },
            || {
                loads.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok::<_, &'static str>(DropProbe {
                    store: std::sync::Arc::downgrade(&store),
                    observed_unlocked_store: std::sync::Arc::clone(&observed_unlocked_store),
                })
            },
        )
        .expect("second probe");

    assert_eq!(second.source, Source::Loaded);
    assert!(observed_unlocked_store.load(std::sync::atomic::Ordering::SeqCst));
    assert_eq!(store.len(), Ok(SINGLE_CAPACITY));
}

#[test]
fn malformed_projection_fails_before_the_loader_runs() {
    let store = Store::<u32>::new(policy(STRICT_LRU_THRESHOLD));
    let loads = std::sync::atomic::AtomicU32::new(0);
    let arguments = arguments("alpha");
    let result = store.lookup_or_load(
        &molten_core::fabric_durability::cache::AccessProjection {
            dataspace_identity: "",
            normalized_arguments: &arguments,
            capability_context: None,
        },
        || {
            loads.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok::<_, &'static str>(FIRST_VALUE)
        },
    );

    assert!(matches!(
        result,
        Err(LookupError::Projection(molten_core::fabric_durability::cache::Issue::EmptyDataspaceIdentity))
    ));
    assert_eq!(loads.load(std::sync::atomic::Ordering::SeqCst), 0);
}
