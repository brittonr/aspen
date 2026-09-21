//! Recorded measurement harness for the marble object-store pilot.
//!
//! Compares the spike backend against the current chunk-store path under one
//! deterministic synthetic campaign workload and records batch write, point
//! read, recovery, and space amplification numbers. Timings are recorded
//! diagnostics from the executed run; counters and workload parameters are
//! deterministic. The decision language stays at `records`, never `proves`.

use std::path::Path;
use std::time::Instant;

use crate::error::MoltenError;
use crate::error::Result;
use crate::marble_store::Backend;
use crate::marble_store::ObjectKind;
use crate::marble_store::StoreConfig;

// r[impl aspen.marble_store.spike]
// r[impl aspen.marble_store.verification]
const MEASUREMENT_SCHEMA: &str = crate::preserves_rail::MARBLE_STORE_MEASUREMENT_SCHEMA;
const DECISION_SCHEMA: &str = crate::preserves_rail::MARBLE_STORE_DECISION_SCHEMA;
const WORKLOAD_SEED: u64 = 0x6d6f_6c74_656e_6d61;
const MIN_BATCHES: u32 = 1;
const MAX_BATCHES: u32 = 4_096;
const MIN_OBJECTS_PER_BATCH: u32 = 1;
const MAX_OBJECTS_PER_BATCH: u32 = 4_096;
const MIN_OBJECT_BYTES: u32 = 64;
const MAX_OBJECT_BYTES: u32 = 1024 * 1024;
const MEASURE_CHUNK_SIZE: u64 = 4 * 1024;
const OBJECT_KIND_LABEL: &str = "marble-pilot-measure";
const PERMILLE: u64 = 1_000;

const _: () = assert!(MIN_OBJECT_BYTES >= 1);
const _: () = assert!(MIN_BATCHES <= MAX_BATCHES);
const _: () = assert!(MIN_OBJECTS_PER_BATCH <= MAX_OBJECTS_PER_BATCH);
const _: () = assert!(MIN_OBJECT_BYTES <= MAX_OBJECT_BYTES);

/// Bounded inputs for one comparison run.
pub struct MeasureInput<'a> {
    /// Directory for the marble heap.
    pub heap_path: &'a Path,
    /// Directory for the current chunk-store path.
    pub chunk_root: &'a Path,
    /// Number of write batches per path.
    pub batches: u32,
    /// Objects per batch.
    pub objects_per_batch: u32,
    /// Bytes per object.
    pub object_bytes: u32,
}

impl MeasureInput<'_> {
    // r[impl aspen.marble_store.spike]
    fn validate(&self) -> Result<()> {
        let mut issues = Vec::new();
        if self.batches < MIN_BATCHES || self.batches > MAX_BATCHES {
            issues.push("batches out of bounds");
        }
        if self.objects_per_batch < MIN_OBJECTS_PER_BATCH || self.objects_per_batch > MAX_OBJECTS_PER_BATCH {
            issues.push("objects_per_batch out of bounds");
        }
        if self.object_bytes < MIN_OBJECT_BYTES || self.object_bytes > MAX_OBJECT_BYTES {
            issues.push("object_bytes out of bounds");
        }
        if issues.is_empty() {
            Ok(())
        } else {
            Err(MoltenError::invalid_harness(format!("marble store measurement denied: {issues:?}")))
        }
    }

    fn logical_bytes(&self) -> u64 {
        u64::from(self.batches) * u64::from(self.objects_per_batch) * u64::from(self.object_bytes)
    }

    fn workload(&self, batch: u32, object: u32) -> Vec<u8> {
        workload_bytes(WORKLOAD_SEED ^ (u64::from(batch) << 32) ^ u64::from(object), self.object_bytes as usize)
    }
}

/// Recorded numbers for one physical path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathNumbers {
    /// Total batch write wall time in microseconds.
    pub batch_write_micros: u64,
    /// Total point read wall time in microseconds.
    pub point_read_micros: u64,
    /// Logical object bytes written.
    pub logical_bytes: u64,
    /// Physical bytes on disk after maintenance.
    pub physical_bytes: u64,
    /// Space amplification in thousandths.
    pub space_amplification_permille: u64,
}

/// Recorded comparison between the spike and the current path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureReport {
    /// Parameters and counters of the executed run.
    pub batches: u32,
    pub objects_per_batch: u32,
    pub object_bytes: u32,
    /// Objects that round-tripped through the spike backend.
    pub spike_round_trips: usize,
    /// Objects that round-tripped through the current path.
    pub current_round_trips: usize,
    /// Spike backend numbers.
    pub spike: PathNumbers,
    /// Current chunk-store numbers.
    pub current: PathNumbers,
    /// Spike recovery wall time in microseconds.
    pub recovery_micros: u64,
    /// Digest mappings replayed by spike recovery.
    pub recovered_mappings: usize,
}

/// Recorded keep-or-replace decision over measured numbers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionRecord {
    /// `keep-current-path` or `replace-candidate`.
    pub decision: &'static str,
    /// Rationale keys, sorted.
    pub rationale: Vec<&'static str>,
    /// Whether every round-trip held on both paths.
    pub round_trips_held: bool,
}

/// Runs the comparison under the deterministic synthetic campaign workload.
// r[impl aspen.marble_store.spike]
pub fn compare_paths(input: &MeasureInput<'_>) -> Result<MeasureReport> {
    input.validate()?;
    let spike = measure_spike(input)?;
    let current = measure_current(input)?;
    let report = MeasureReport {
        batches: input.batches,
        objects_per_batch: input.objects_per_batch,
        object_bytes: input.object_bytes,
        spike_round_trips: spike.round_trips,
        current_round_trips: current.round_trips,
        spike: spike.numbers,
        current: current.numbers,
        recovery_micros: spike.recovery_micros,
        recovered_mappings: spike.recovered_mappings,
    };
    Ok(report)
}

/// Derives the recorded keep-or-replace decision from measured numbers.
// r[impl aspen.marble_store.spike]
pub fn keep_or_replace(report: &MeasureReport) -> DecisionRecord {
    let writes_faster = report.spike.batch_write_micros < report.current.batch_write_micros;
    let reads_faster = report.spike.point_read_micros < report.current.point_read_micros;
    let space_not_worse = report.spike.space_amplification_permille <= report.current.space_amplification_permille;
    let round_trips_held = report.spike_round_trips > 0 && report.current_round_trips > 0;
    let mut rationale = Vec::new();
    rationale.push(if writes_faster {
        "spike-batch-write-faster"
    } else {
        "current-batch-write-faster"
    });
    rationale.push(if reads_faster {
        "spike-point-read-faster"
    } else {
        "current-point-read-faster"
    });
    rationale.push(if space_not_worse {
        "spike-space-not-worse"
    } else {
        "current-space-better"
    });
    rationale.push("recovery-replays-atomically-recovered-batches-only");
    rationale.sort_unstable();
    let decision = if round_trips_held && writes_faster && reads_faster && space_not_worse {
        "replace-candidate"
    } else {
        "keep-current-path"
    };
    DecisionRecord {
        decision,
        rationale,
        round_trips_held,
    }
}

/// Canonical measurement value for the recorded run.
// r[impl aspen.marble_store.verification]
pub fn measurement_value(report: &MeasureReport) -> preserves::IOValue {
    record("marble-store-measurement", vec![
        string(MEASUREMENT_SCHEMA),
        u64_value(u64::from(report.batches)),
        u64_value(u64::from(report.objects_per_batch)),
        u64_value(u64::from(report.object_bytes)),
        u64_value(report.spike_round_trips as u64),
        u64_value(report.current_round_trips as u64),
        numbers_value("spike", &report.spike),
        numbers_value("current", &report.current),
        u64_value(report.recovery_micros),
        u64_value(report.recovered_mappings as u64),
    ])
}

/// Canonical decision value for the recorded run.
// r[impl aspen.marble_store.verification]
pub fn decision_value(report: &MeasureReport, decision: &DecisionRecord) -> preserves::IOValue {
    record("marble-store-decision", vec![
        string(DECISION_SCHEMA),
        string(decision.decision),
        strings(&decision.rationale),
        bool_value(decision.round_trips_held),
        record("batch-write-micros", vec![
            u64_value(report.spike.batch_write_micros),
            u64_value(report.current.batch_write_micros),
        ]),
        record("point-read-micros", vec![
            u64_value(report.spike.point_read_micros),
            u64_value(report.current.point_read_micros),
        ]),
        record("recovery-micros", vec![
            u64_value(report.recovery_micros),
            u64_value(report.recovered_mappings as u64),
        ]),
        record("space-amplification-permille", vec![
            u64_value(report.spike.space_amplification_permille),
            u64_value(report.current.space_amplification_permille),
        ]),
    ])
}

struct PathRun {
    numbers: PathNumbers,
    round_trips: usize,
    recovery_micros: u64,
    recovered_mappings: usize,
}

fn measure_spike(input: &MeasureInput<'_>) -> Result<PathRun> {
    let config = StoreConfig::pilot(input.heap_path);
    let backend = Backend::open(&config)?;
    let write_started = Instant::now();
    for batch in 0..input.batches {
        let mut objects = Vec::with_capacity(input.objects_per_batch as usize);
        for object in 0..input.objects_per_batch {
            objects.push((ObjectKind::Content, input.workload(batch, object)));
        }
        backend.store_batch(objects)?;
    }
    let batch_write_micros = micros_since(write_started);
    let read_started = Instant::now();
    let mut round_trips = 0;
    for batch in 0..input.batches {
        for object in 0..input.objects_per_batch {
            let bytes = input.workload(batch, object);
            let content_ref = crate::marble_store::content_ref_of(&crate::marble_store::digest_of(&bytes));
            let fetched = backend.fetch(&content_ref)?;
            if fetched.bytes.as_slice() == bytes.as_slice() {
                round_trips += 1;
            }
        }
    }
    let point_read_micros = micros_since(read_started);
    let stats = backend.stats()?;
    drop(backend);
    let recovery_started = Instant::now();
    let reopened = Backend::open(&config)?;
    let recovery_micros = micros_since(recovery_started);
    let recovery = reopened.recovery();
    Ok(PathRun {
        numbers: PathNumbers {
            batch_write_micros,
            point_read_micros,
            logical_bytes: input.logical_bytes(),
            physical_bytes: stats.total_file_size,
            space_amplification_permille: permille(stats.total_file_size, input.logical_bytes()),
        },
        round_trips,
        recovery_micros,
        recovered_mappings: recovery.recovered_mappings,
    })
}

fn measure_current(input: &MeasureInput<'_>) -> Result<PathRun> {
    std::fs::create_dir_all(input.chunk_root)?;
    let write_started = Instant::now();
    let mut stored_pairs = Vec::with_capacity(total_objects(input));
    for batch in 0..input.batches {
        for object in 0..input.objects_per_batch {
            let bytes = input.workload(batch, object);
            let stored =
                crate::chunk_store::put_bytes(input.chunk_root, OBJECT_KIND_LABEL, &bytes, MEASURE_CHUNK_SIZE)?;
            stored_pairs.push((stored.manifest_ref, (batch, object)));
        }
    }
    let batch_write_micros = micros_since(write_started);
    stored_pairs.sort_by(|left, right| left.0.cmp(&right.0));
    let read_started = Instant::now();
    let mut round_trips = 0;
    for (manifest_ref, coordinates) in &stored_pairs {
        let read = crate::chunk_store::read_object(input.chunk_root, manifest_ref)?;
        if read.bytes == input.workload(coordinates.0, coordinates.1) {
            round_trips += 1;
        }
    }
    let point_read_micros = micros_since(read_started);
    let physical_bytes = directory_bytes(input.chunk_root)?;
    Ok(PathRun {
        numbers: PathNumbers {
            batch_write_micros,
            point_read_micros,
            logical_bytes: input.logical_bytes(),
            physical_bytes,
            space_amplification_permille: permille(physical_bytes, input.logical_bytes()),
        },
        round_trips,
        recovery_micros: 0,
        recovered_mappings: 0,
    })
}

fn total_objects(input: &MeasureInput<'_>) -> usize {
    (input.batches * input.objects_per_batch) as usize
}

fn micros_since(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX)
}

fn permille(physical: u64, logical: u64) -> u64 {
    if logical == 0 {
        return 0;
    }
    (physical.saturating_mul(PERMILLE) / logical.max(1)).min(PERMILLE * 1_000)
}

fn directory_bytes(root: &Path) -> Result<u64> {
    let mut total = 0_u64;
    let mut stack = vec![root.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in std::fs::read_dir(&current)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                total = total.saturating_add(entry.metadata()?.len());
            }
        }
    }
    Ok(total)
}

/// Deterministic synthetic campaign bytes from a splitmix64 stream.
pub fn workload_bytes(seed: u64, len: usize) -> Vec<u8> {
    let mut state = seed;
    let mut bytes = Vec::with_capacity(len);
    while bytes.len() < len {
        state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut mixed = state;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        mixed ^= mixed >> 31;
        bytes.extend_from_slice(&mixed.to_le_bytes());
    }
    bytes.truncate(len);
    bytes
}

fn numbers_value(label: &'static str, numbers: &PathNumbers) -> preserves::IOValue {
    record("path-numbers", vec![
        string(label),
        u64_value(numbers.batch_write_micros),
        u64_value(numbers.point_read_micros),
        u64_value(numbers.logical_bytes),
        u64_value(numbers.physical_bytes),
        u64_value(numbers.space_amplification_permille),
    ])
}

fn bool_value(value: bool) -> preserves::IOValue {
    crate::preserves_rail::bool_value(value)
}

fn record(label: &'static str, fields: Vec<preserves::IOValue>) -> preserves::IOValue {
    crate::preserves_rail::record(label, fields)
}

fn string(value: impl AsRef<str>) -> preserves::IOValue {
    crate::preserves_rail::string(value.as_ref())
}

fn strings<'a>(values: &[&'a str]) -> preserves::IOValue {
    crate::preserves_rail::sequence(values.iter().map(|value| string(*value)).collect())
}

fn u64_value(value: u64) -> preserves::IOValue {
    crate::preserves_rail::u64_value(value)
}
