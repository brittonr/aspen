//! Baseline measurement harness for Aspen Snix blob chunking.
//!
//! This is intentionally a small, deterministic, dependency-light bench binary
//! rather than a Criterion benchmark. It produces a JSON receipt suitable for
//! OpenSpec evidence and compares future candidate chunkers against the current
//! FastCDC default without changing production behavior.

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;

use aspen_snix::chunking;
use aspen_snix::chunking::ChunkerAlgorithm;
use serde_json::json;

const MAX_FILES_PER_CLASS: usize = 16;
const MAX_BYTES_PER_FILE: usize = 2 * 1024 * 1024;
const MAX_SCAN_DIR_ENTRIES: usize = 256;
const SYNTHETIC_BASE_SIZE: usize = 512 * 1024;
const SYNTHETIC_DELTA_VARIANTS: usize = 8;

#[derive(Debug)]
struct CorpusFile {
    class: &'static str,
    name: String,
    data: Vec<u8>,
}

#[derive(Default)]
struct CorpusBuilder {
    files: Vec<CorpusFile>,
}

impl CorpusBuilder {
    fn push_file(&mut self, class: &'static str, name: impl Into<String>, data: Vec<u8>) {
        if data.is_empty() {
            return;
        }
        self.files.push(CorpusFile {
            class,
            name: name.into(),
            data,
        });
    }

    fn collect_regular_files(&mut self, class: &'static str, root: &Path, max_files: usize) -> io::Result<()> {
        let mut candidates = Vec::new();
        collect_paths(root, &mut candidates, MAX_SCAN_DIR_ENTRIES)?;
        candidates.sort();

        let mut added = 0usize;
        for path in candidates {
            if added >= max_files {
                break;
            }
            let metadata = match fs::metadata(&path) {
                Ok(metadata) => metadata,
                Err(_) => continue,
            };
            if !metadata.is_file() || metadata.len() == 0 {
                continue;
            }
            let data = match fs::read(&path) {
                Ok(mut data) => {
                    data.truncate(MAX_BYTES_PER_FILE);
                    data
                }
                Err(_) => continue,
            };
            self.push_file(class, display_path(&path), data);
            added += 1;
        }
        Ok(())
    }
}

fn collect_paths(root: &Path, out: &mut Vec<PathBuf>, max_entries: usize) -> io::Result<()> {
    if out.len() >= max_entries {
        return Ok(());
    }
    let metadata = match fs::metadata(root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    if metadata.is_file() {
        out.push(root.to_path_buf());
        return Ok(());
    }
    if !metadata.is_dir() {
        return Ok(());
    }

    let mut entries = fs::read_dir(root)?.filter_map(Result::ok).map(|e| e.path()).collect::<Vec<_>>();
    entries.sort();
    for path in entries.into_iter().take(max_entries) {
        if out.len() >= max_entries {
            break;
        }
        let _ = collect_paths(&path, out, max_entries);
    }
    Ok(())
}

fn display_path(path: &Path) -> String {
    path.strip_prefix(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .unwrap_or(path)
        .display()
        .to_string()
}

fn synthetic_base() -> Vec<u8> {
    (0..SYNTHETIC_BASE_SIZE)
        .map(|i| {
            let mixed = (i as u32).wrapping_mul(2_654_435_761).rotate_left((i % 31) as u32);
            (mixed ^ (mixed >> 11)) as u8
        })
        .collect()
}

fn add_synthetic_delta_corpus(builder: &mut CorpusBuilder) {
    let base = synthetic_base();
    builder.push_file("synthetic-small-delta", "synthetic/base", base.clone());

    for variant in 0..SYNTHETIC_DELTA_VARIANTS {
        let mut mutated = base.clone();
        let insert_at = 4096 + variant * 8192;
        let insert_len = 1024 + variant * 257;
        let insert =
            (0..insert_len).map(|i| (variant as u8).wrapping_mul(17).wrapping_add(i as u8)).collect::<Vec<_>>();
        mutated.splice(insert_at..insert_at, insert);

        for byte in mutated.iter_mut().skip(128 * 1024 + variant * 4096).take(256) {
            *byte = byte.wrapping_add(variant as u8 + 1);
        }

        builder.push_file("synthetic-small-delta", format!("synthetic/delta-{variant}"), mutated);
    }
}

fn collect_corpus() -> Vec<CorpusFile> {
    let mut builder = CorpusBuilder::default();
    add_synthetic_delta_corpus(&mut builder);

    if let Ok(paths) = std::env::var("ASPEN_SNIX_BENCH_NIX_STORE_PATHS") {
        for path in paths.split(':').filter(|p| !p.is_empty()) {
            let _ = builder.collect_regular_files("nix-store", Path::new(path), MAX_FILES_PER_CLASS);
        }
    } else {
        let _ = builder.collect_regular_files(
            "nix-store",
            Path::new("/run/current-system/sw/bin"),
            MAX_FILES_PER_CLASS / 2,
        );
        let _ = builder.collect_regular_files("nix-store", Path::new("/nix/store"), MAX_FILES_PER_CLASS / 2);
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().and_then(Path::parent).unwrap_or(&manifest_dir);
    let _ = builder.collect_regular_files(
        "aspen-build-artifact",
        &workspace_root.join("target/debug"),
        MAX_FILES_PER_CLASS / 2,
    );
    let _ = builder.collect_regular_files(
        "aspen-build-artifact",
        &workspace_root.join("target/release"),
        MAX_FILES_PER_CLASS / 2,
    );

    if let Ok(current_exe) = std::env::current_exe() {
        if let Ok(mut data) = fs::read(&current_exe) {
            data.truncate(MAX_BYTES_PER_FILE);
            builder.push_file("aspen-build-artifact", display_path(&current_exe), data);
        }
    }

    builder.files
}

fn percentile(sorted: &[u32], numerator: usize, denominator: usize) -> u32 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((sorted.len() - 1) * numerator) / denominator;
    sorted[idx]
}

fn parse_algorithm() -> ChunkerAlgorithm {
    match std::env::var("ASPEN_SNIX_CHUNKER") {
        Ok(value) if value == ChunkerAlgorithm::ExperimentalHashlessCdc.as_str() || value == "experimental" => {
            ChunkerAlgorithm::ExperimentalHashlessCdc
        }
        Ok(value) if value == ChunkerAlgorithm::FastCdc.as_str() || value == "fastcdc" => ChunkerAlgorithm::FastCdc,
        Ok(value) => panic!(
            "unsupported ASPEN_SNIX_CHUNKER '{value}'; expected '{}' or '{}'",
            ChunkerAlgorithm::FastCdc.as_str(),
            ChunkerAlgorithm::ExperimentalHashlessCdc.as_str()
        ),
        Err(_) => ChunkerAlgorithm::FastCdc,
    }
}

fn main() {
    let algorithm = parse_algorithm();
    let corpus = collect_corpus();
    let started = Instant::now();

    let mut total_bytes = 0u64;
    let mut total_chunks = 0usize;
    let mut unique_chunks = HashSet::new();
    let mut chunk_sizes = Vec::new();
    let mut class_counts = std::collections::BTreeMap::<&'static str, (usize, u64)>::new();
    let mut file_reports = Vec::new();

    for file in &corpus {
        let file_started = Instant::now();
        let chunks = chunking::chunk_blob_with_algorithm(&file.data, algorithm)
            .unwrap_or_else(|error| panic!("chunker '{}' failed for '{}': {error}", algorithm.as_str(), file.name));
        let file_elapsed = file_started.elapsed();
        let class_entry = class_counts.entry(file.class).or_default();
        class_entry.0 += 1;
        class_entry.1 += file.data.len() as u64;
        total_bytes += file.data.len() as u64;
        total_chunks += chunks.len();

        let mut file_unique = HashSet::new();
        for chunk in &chunks {
            unique_chunks.insert(*chunk.hash.as_bytes());
            file_unique.insert(*chunk.hash.as_bytes());
            chunk_sizes.push(chunk.size);
        }

        file_reports.push(json!({
            "class": file.class,
            "name": file.name,
            "bytes": file.data.len(),
            "chunks": chunks.len(),
            "unique_chunks_in_file": file_unique.len(),
            "elapsed_micros": file_elapsed.as_micros(),
        }));
    }

    chunk_sizes.sort_unstable();
    let elapsed = started.elapsed();
    let elapsed_secs = elapsed.as_secs_f64().max(0.000_001);
    let duplicate_chunks = total_chunks.saturating_sub(unique_chunks.len());
    let dedup_ratio = if total_chunks == 0 {
        0.0
    } else {
        duplicate_chunks as f64 / total_chunks as f64
    };

    let classes = class_counts
        .iter()
        .map(|(class, (files, bytes))| json!({ "class": class, "files": files, "bytes": bytes }))
        .collect::<Vec<_>>();

    let report = json!({
        "algorithm": algorithm.as_str(),
        "parameters": {
            "min_chunk_size": chunking::MIN_CHUNK_SIZE,
            "avg_chunk_size": chunking::AVG_CHUNK_SIZE,
            "max_chunk_size": chunking::MAX_CHUNK_SIZE,
            "inline_threshold": chunking::INLINE_THRESHOLD,
        },
        "limits": {
            "max_files_per_class": MAX_FILES_PER_CLASS,
            "max_bytes_per_file": MAX_BYTES_PER_FILE,
            "max_scan_dir_entries": MAX_SCAN_DIR_ENTRIES,
        },
        "corpus": {
            "files": corpus.len(),
            "bytes": total_bytes,
            "classes": classes,
        },
        "results": {
            "wall_micros": elapsed.as_micros(),
            "throughput_mib_per_sec": (total_bytes as f64 / (1024.0 * 1024.0)) / elapsed_secs,
            "total_chunks": total_chunks,
            "unique_chunks": unique_chunks.len(),
            "duplicate_chunks": duplicate_chunks,
            "dedup_ratio": dedup_ratio,
            "chunk_size_min": chunk_sizes.first().copied().unwrap_or(0),
            "chunk_size_p50": percentile(&chunk_sizes, 50, 100),
            "chunk_size_p95": percentile(&chunk_sizes, 95, 100),
            "chunk_size_max": chunk_sizes.last().copied().unwrap_or(0),
        },
        "files": file_reports,
    });

    println!("{}", serde_json::to_string_pretty(&report).expect("serialize report"));
}
