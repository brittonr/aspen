use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

static LIVE: AtomicU64 = AtomicU64::new(0);
static PEAK: AtomicU64 = AtomicU64::new(0);
static ALLOCS: AtomicU64 = AtomicU64::new(0);
static ALLOC_BYTES: AtomicU64 = AtomicU64::new(0);
static FREES: AtomicU64 = AtomicU64::new(0);
struct Counting;
fn allocated(size: usize) {
    ALLOCS.fetch_add(1, Ordering::Relaxed);
    ALLOC_BYTES.fetch_add(size as u64, Ordering::Relaxed);
    let live = LIVE.fetch_add(size as u64, Ordering::Relaxed) + size as u64;
    PEAK.fetch_max(live, Ordering::Relaxed);
}
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() { allocated(layout.size()); }
        pointer
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() { allocated(layout.size()); }
        pointer
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
        LIVE.fetch_sub(layout.size() as u64, Ordering::Relaxed);
        FREES.fetch_add(1, Ordering::Relaxed);
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let next = unsafe { System.realloc(pointer, layout, size) };
        if !next.is_null() {
            LIVE.fetch_sub(layout.size() as u64, Ordering::Relaxed);
            FREES.fetch_add(1, Ordering::Relaxed);
            allocated(size);
        }
        next
    }
}
#[cfg(feature = "allocation-counters")]
#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn rss_kib() -> u64 {
    let status = std::fs::read_to_string("/proc/self/status").expect("Linux RSS observation");
    status.lines().find_map(|line| line.strip_prefix("VmRSS:")).expect("VmRSS")
        .split_whitespace().next().expect("RSS value").parse().expect("RSS integer")
}
fn measure<T>(variant: &str, fixture: &str, phase: &str, f: impl FnOnce() -> T) {
    let rss_before = rss_kib();
    let initial = LIVE.load(Ordering::Relaxed);
    let allocations = ALLOCS.load(Ordering::Relaxed);
    let bytes = ALLOC_BYTES.load(Ordering::Relaxed);
    let frees = FREES.load(Ordering::Relaxed);
    PEAK.store(initial, Ordering::Relaxed);
    let start = Instant::now();
    let output = f();
    std::hint::black_box(&output);
    drop(output);
    let elapsed = start.elapsed().as_nanos();
    let peak = PEAK.load(Ordering::Relaxed).saturating_sub(initial);
    let retained = i128::from(LIVE.load(Ordering::Relaxed)) - i128::from(initial);
    let allocations = ALLOCS.load(Ordering::Relaxed) - allocations;
    let bytes = ALLOC_BYTES.load(Ordering::Relaxed) - bytes;
    let frees = FREES.load(Ordering::Relaxed) - frees;
    let rss_after = rss_kib();
    println!("{variant}\t{fixture}\t{phase}\t{elapsed}\t{allocations}\t{bytes}\t{frees}\t{peak}\t{retained}\t{rss_before}\t{rss_after}");
}
mod current { use current_core::prolly_map::*; include!("runner.rs"); }
mod candidate { use candidate_core::prolly_map::*; include!("runner.rs"); }
mod sorted { use sorted_core::prolly_map::*; include!("runner.rs"); }

fn byte_api_intake() {
    use std::collections::BTreeMap;
    use bbtree::unsync::BBTreeMap;
    let mut keys = vec![vec![], vec![0], vec![0,0], vec![0,255], vec![127], vec![128], vec![255]];
    for length in 1..=64 { keys.push(vec![0x80; length]); }
    for index in 0_u32..4096 {
        let mut key = vec![0x61; 64];
        key[60..].copy_from_slice(&index.to_be_bytes());
        keys.push(key);
    }
    // Each batch fits the 4096-entry profile. Empty keys are admitted by the map
    // API, but the real Prolly planner must continue to reject them.
    for group in keys.chunks(4096) {
        let mut reference = BTreeMap::new();
        let mut candidate = BBTreeMap::new();
        for (index, key) in group.iter().rev().enumerate() {
            let value = vec![index as u8; index % 1025];
            assert_eq!(candidate.insert(key.clone(), value.clone()), reference.insert(key.clone(), value));
        }
        assert_eq!(candidate.iter().collect::<Vec<_>>(), reference.iter().collect::<Vec<_>>());
        for key in group {
            assert_eq!(candidate.get(key), reference.get(key));
            assert_eq!(candidate.get(key.as_slice()), reference.get(key.as_slice()));
            assert_eq!(candidate.get_mut(key).map(|v| v.len()), reference.get_mut(key).map(|v| v.len()));
        }
        let cloned = candidate.clone();
        assert_eq!(cloned.into_iter().collect::<Vec<_>>(), reference.clone().into_iter().collect::<Vec<_>>());
        for key in group { assert_eq!(candidate.remove(key), reference.remove(key)); }
        assert!(candidate.is_empty());
    }
    // Every unsigned byte at every admitted byte offset, and prefix ties at 0.
    for length in 1..=64 {
        let mut candidate = BBTreeMap::new();
        let mut reference = BTreeMap::new();
        for byte in 0_u8..=255 {
            let mut key = vec![0; length];
            key[length-1] = byte;
            candidate.insert(key.clone(), byte);
            reference.insert(key, byte);
        }
        assert_eq!(candidate.into_iter().collect::<Vec<_>>(), reference.into_iter().collect::<Vec<_>>());
    }
    println!("byte-api-intake ok (sampled, not exhaustive or a proof)");
}
fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    match args.get(1).map(String::as_str) {
        Some("semantics") => {
            byte_api_intake();
            assert_eq!(current::case_names(), candidate::case_names());
            assert_eq!(current::case_names(), sorted::case_names());
            for name in current::case_names() {
                let baseline = current::semantic_case(name);
                let candidate = candidate::semantic_case(name);
                assert_eq!(baseline, candidate, "full candidate observations differ: {name}");
                drop(candidate);
                let sorted = sorted::semantic_case(name);
                assert_eq!(baseline, sorted, "full sorted observations differ: {name}");
                println!("semantic-equivalence ok {name}");
            }
        }
        Some("measure") => {
            let variant = args.get(2).expect("variant current|candidate|sorted");
            println!("variant\tfixture\tphase\tns\tallocations\tallocated_bytes\tfrees\tpeak_extra_live_bytes\tretained_live_bytes\trss_before_kib\trss_after_kib");
            match variant.as_str() {
                "current" => current::measurements(variant),
                "candidate" => candidate::measurements(variant),
                "sorted" => sorted::measurements(variant),
                _ => panic!("unknown variant"),
            }
        }
        _ => panic!("usage: issue3-probe semantics | measure current|candidate|sorted"),
    }
}
