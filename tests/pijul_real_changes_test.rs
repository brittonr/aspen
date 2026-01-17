//! Pijul Real Changes Integration Tests
//!
//! Tests that exercise the full record -> store -> sync -> apply -> output
//! cycle with actual Pijul patches. These tests verify the complete pipeline
//! works correctly for real-world scenarios.
//!
//! # Test Categories
//!
//! - Full Pipeline: Single-node tests of the complete workflow
//! - Dependency Chains: Verify change dependencies are preserved
//! - Edge Cases: Binary files, unicode, large files, special characters
//!
//! # Running These Tests
//!
//! ```bash
//! # Run all real changes tests (requires network for multi-node)
//! cargo nextest run -E 'test(/pijul_real_changes/)' --features pijul --run-ignored all
//!
//! # Run only single-node tests (no network required)
//! cargo nextest run -E 'test(/pijul_real_changes/)' --features pijul
//! ```

#![cfg(feature = "pijul")]

use std::sync::Arc;

use aspen::blob::InMemoryBlobStore;
use aspen::forge::identity::RepoId;
use aspen::pijul::AspenChangeStore;
use aspen::pijul::ChangeApplicator;
use aspen::pijul::ChangeDirectory;
use aspen::pijul::ChangeRecorder;
use aspen::pijul::PristineManager;
use aspen::pijul::WorkingDirOutput;
use tempfile::TempDir;

/// Helper to create test infrastructure.
struct RealChangesTestSetup {
    tmp: TempDir,
    data_dir: std::path::PathBuf,
    repo_id: RepoId,
    #[allow(dead_code)]
    blobs: Arc<InMemoryBlobStore>,
    change_store: Arc<AspenChangeStore<InMemoryBlobStore>>,
    pristine_mgr: PristineManager,
}

impl RealChangesTestSetup {
    fn new(name: &str) -> Self {
        let tmp = TempDir::new().unwrap();
        let data_dir = tmp.path().join("data");
        let repo_id = RepoId::from_hash(blake3::hash(name.as_bytes()));

        let blobs = Arc::new(InMemoryBlobStore::new());
        let change_store = Arc::new(AspenChangeStore::new(blobs.clone()));
        let pristine_mgr = PristineManager::new(&data_dir);

        Self {
            tmp,
            data_dir,
            repo_id,
            blobs,
            change_store,
            pristine_mgr,
        }
    }

    fn work_dir(&self, name: &str) -> std::path::PathBuf {
        let path = self.tmp.path().join(name);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn change_dir(&self) -> ChangeDirectory<InMemoryBlobStore> {
        ChangeDirectory::new(&self.data_dir, self.repo_id, self.change_store.clone())
    }
}

// ============================================================================
// Full Pipeline Tests
// ============================================================================

/// Test: Full pipeline with a single file.
///
/// Record -> Store -> Apply -> Output -> Verify
#[tokio::test]
async fn test_pijul_real_changes_full_pipeline_single_file() {
    let setup = RealChangesTestSetup::new("full-pipeline-single");
    let work_dir = setup.work_dir("work");

    // Create a file
    let content = "Hello, Pijul Real Changes!\nThis tests the full pipeline.\n";
    std::fs::write(work_dir.join("test.txt"), content).unwrap();

    // Create pristine and recorder
    let pristine = setup.pristine_mgr.open_or_create(&setup.repo_id).unwrap();
    let change_dir = setup.change_dir();
    let recorder = ChangeRecorder::new(pristine.clone(), change_dir.clone(), work_dir);

    // Record the change
    let result = recorder.record("main", "Add test file", "Test <test@test.com>").await.unwrap();
    assert!(result.is_some(), "should have changes to record");
    let recorded = result.unwrap();

    println!("Recorded: hash={}, hunks={}, size={}", recorded.hash, recorded.num_hunks, recorded.size_bytes);

    // Verify the change is stored
    let stored = setup.change_store.has_change(&recorded.hash).await.unwrap();
    assert!(stored, "change should be stored");

    // Output to a new directory and verify
    let output_dir = setup.work_dir("output");
    let outputter = WorkingDirOutput::new(pristine.clone(), change_dir.clone(), output_dir.clone());
    let output_result = outputter.output("main").unwrap();

    assert!(output_result.is_clean(), "should have no conflicts");

    // Verify the content
    let output_content = std::fs::read_to_string(output_dir.join("test.txt")).unwrap();
    assert_eq!(output_content, content, "output content should match input");
}

/// Test: Full pipeline with directory structure.
///
/// Verifies nested directories are properly recorded and output.
#[tokio::test]
async fn test_pijul_real_changes_full_pipeline_directory_structure() {
    let setup = RealChangesTestSetup::new("full-pipeline-dirs");
    let work_dir = setup.work_dir("work");

    // Create nested structure
    std::fs::create_dir_all(work_dir.join("src/core")).unwrap();
    std::fs::create_dir_all(work_dir.join("tests")).unwrap();
    std::fs::write(work_dir.join("README.md"), "# Project\n").unwrap();
    std::fs::write(work_dir.join("src/lib.rs"), "mod core;\n").unwrap();
    std::fs::write(work_dir.join("src/core/mod.rs"), "pub fn init() {}\n").unwrap();
    std::fs::write(work_dir.join("tests/basic.rs"), "#[test] fn test() {}\n").unwrap();

    // Record
    let pristine = setup.pristine_mgr.open_or_create(&setup.repo_id).unwrap();
    let change_dir = setup.change_dir();
    let recorder = ChangeRecorder::new(pristine.clone(), change_dir.clone(), work_dir);

    let result = recorder.record("main", "Initial structure", "Test <test@test.com>").await.unwrap();
    assert!(result.is_some());
    let recorded = result.unwrap();

    println!("Recorded directory structure: {} hunks", recorded.num_hunks);

    // Output and verify all files exist
    let output_dir = setup.work_dir("output");
    let outputter = WorkingDirOutput::new(pristine, change_dir, output_dir.clone());
    outputter.output("main").unwrap();

    assert!(output_dir.join("README.md").exists());
    assert!(output_dir.join("src/lib.rs").exists());
    assert!(output_dir.join("src/core/mod.rs").exists());
    assert!(output_dir.join("tests/basic.rs").exists());

    // Verify content
    let lib_content = std::fs::read_to_string(output_dir.join("src/lib.rs")).unwrap();
    assert_eq!(lib_content, "mod core;\n");
}

/// Test: Full pipeline with file modification.
///
/// Verifies modifications are correctly recorded and output.
#[tokio::test]
async fn test_pijul_real_changes_full_pipeline_file_modification() {
    let setup = RealChangesTestSetup::new("full-pipeline-modify");
    let work_dir = setup.work_dir("work");

    // Create initial file
    std::fs::write(work_dir.join("counter.txt"), "count: 0\n").unwrap();

    let pristine = setup.pristine_mgr.open_or_create(&setup.repo_id).unwrap();
    let change_dir = setup.change_dir();

    // Record initial
    let recorder = ChangeRecorder::new(pristine.clone(), change_dir.clone(), work_dir.clone());
    let first = recorder.record("main", "Initial", "Test <test@test.com>").await.unwrap().unwrap();

    println!("First change: {}", first.hash);

    // Modify file
    std::fs::write(work_dir.join("counter.txt"), "count: 1\n").unwrap();

    // Record modification
    let recorder2 = ChangeRecorder::new(pristine.clone(), change_dir.clone(), work_dir);
    let second = recorder2.record("main", "Increment", "Test <test@test.com>").await.unwrap().unwrap();

    println!("Second change: {}", second.hash);

    assert_ne!(first.hash, second.hash);

    // Output final state
    let output_dir = setup.work_dir("output");
    let outputter = WorkingDirOutput::new(pristine, change_dir, output_dir.clone());
    outputter.output("main").unwrap();

    let content = std::fs::read_to_string(output_dir.join("counter.txt")).unwrap();
    assert_eq!(content, "count: 1\n", "should have modified content");
}

// ============================================================================
// Dependency Chain Tests
// ============================================================================

/// Test: Linear dependency chain (A -> B -> C).
///
/// Changes must be applied in order.
#[tokio::test]
async fn test_pijul_real_changes_dependency_chain_linear() {
    let setup = RealChangesTestSetup::new("dep-chain-linear");
    let work_dir = setup.work_dir("work");

    let pristine = setup.pristine_mgr.open_or_create(&setup.repo_id).unwrap();
    let change_dir = setup.change_dir();

    // Change A: Create file
    std::fs::write(work_dir.join("history.txt"), "Change A\n").unwrap();
    let recorder = ChangeRecorder::new(pristine.clone(), change_dir.clone(), work_dir.clone());
    let change_a = recorder.record("main", "Change A", "Test <test@test.com>").await.unwrap().unwrap();

    // Change B: Append to file
    std::fs::write(work_dir.join("history.txt"), "Change A\nChange B\n").unwrap();
    let recorder = ChangeRecorder::new(pristine.clone(), change_dir.clone(), work_dir.clone());
    let change_b = recorder.record("main", "Change B", "Test <test@test.com>").await.unwrap().unwrap();

    // Change C: Append again
    std::fs::write(work_dir.join("history.txt"), "Change A\nChange B\nChange C\n").unwrap();
    let recorder = ChangeRecorder::new(pristine.clone(), change_dir.clone(), work_dir);
    let change_c = recorder.record("main", "Change C", "Test <test@test.com>").await.unwrap().unwrap();

    println!("Chain: {} -> {} -> {}", change_a.hash, change_b.hash, change_c.hash);

    // Verify all changes stored
    assert!(setup.change_store.has_change(&change_a.hash).await.unwrap());
    assert!(setup.change_store.has_change(&change_b.hash).await.unwrap());
    assert!(setup.change_store.has_change(&change_c.hash).await.unwrap());

    // Output and verify final state includes all changes
    let output_dir = setup.work_dir("output");
    let outputter = WorkingDirOutput::new(pristine, change_dir, output_dir.clone());
    outputter.output("main").unwrap();

    let content = std::fs::read_to_string(output_dir.join("history.txt")).unwrap();
    assert_eq!(content, "Change A\nChange B\nChange C\n");
}

/// Test: Branching dependencies (A -> B, A -> C).
///
/// Multiple changes depend on the same parent.
#[tokio::test]
async fn test_pijul_real_changes_dependency_chain_branching() {
    let setup = RealChangesTestSetup::new("dep-chain-branch");
    let work_dir = setup.work_dir("work");

    let pristine = setup.pristine_mgr.open_or_create(&setup.repo_id).unwrap();
    let change_dir = setup.change_dir();

    // Change A: Create base file
    std::fs::write(work_dir.join("base.txt"), "Base content\n").unwrap();
    let recorder = ChangeRecorder::new(pristine.clone(), change_dir.clone(), work_dir.clone());
    let change_a = recorder.record("main", "Change A", "Test <test@test.com>").await.unwrap().unwrap();

    println!("Base change: {}", change_a.hash);

    // Fork to branch channel for change B
    {
        let mut txn = pristine.mut_txn_begin().unwrap();
        txn.fork_channel("main", "branch-b").unwrap();
        txn.commit().unwrap();
    }

    // Change B on branch-b: Add feature file
    std::fs::write(work_dir.join("feature_b.txt"), "Feature B\n").unwrap();
    let recorder = ChangeRecorder::new(pristine.clone(), change_dir.clone(), work_dir.clone());
    let change_b = recorder.record("branch-b", "Change B", "Test <test@test.com>").await.unwrap().unwrap();

    println!("Branch B change: {}", change_b.hash);

    // Fork again from main for change C
    {
        let mut txn = pristine.mut_txn_begin().unwrap();
        txn.fork_channel("main", "branch-c").unwrap();
        txn.commit().unwrap();
    }

    // Change C on branch-c: Add different file
    std::fs::write(work_dir.join("feature_c.txt"), "Feature C\n").unwrap();
    let recorder = ChangeRecorder::new(pristine.clone(), change_dir.clone(), work_dir);
    let change_c = recorder.record("branch-c", "Change C", "Test <test@test.com>").await.unwrap().unwrap();

    println!("Branch C change: {}", change_c.hash);

    // Both branches should have the base file
    let output_b = setup.work_dir("output_b");
    let outputter_b = WorkingDirOutput::new(pristine.clone(), change_dir.clone(), output_b.clone());
    outputter_b.output("branch-b").unwrap();
    assert!(output_b.join("base.txt").exists());
    assert!(output_b.join("feature_b.txt").exists());

    let output_c = setup.work_dir("output_c");
    let outputter_c = WorkingDirOutput::new(pristine.clone(), change_dir.clone(), output_c.clone());
    outputter_c.output("branch-c").unwrap();
    assert!(output_c.join("base.txt").exists());
    assert!(output_c.join("feature_c.txt").exists());
}

// ============================================================================
// Edge Case Tests
// ============================================================================

/// Test: Empty file handling.
#[tokio::test]
async fn test_pijul_real_changes_empty_file_handling() {
    let setup = RealChangesTestSetup::new("empty-file");
    let work_dir = setup.work_dir("work");

    // Create empty file
    std::fs::write(work_dir.join("empty.txt"), "").unwrap();

    let pristine = setup.pristine_mgr.open_or_create(&setup.repo_id).unwrap();
    let change_dir = setup.change_dir();
    let recorder = ChangeRecorder::new(pristine.clone(), change_dir.clone(), work_dir);

    let result = recorder.record("main", "Add empty file", "Test <test@test.com>").await.unwrap();
    assert!(result.is_some(), "should record empty file");
    let recorded = result.unwrap();

    println!("Empty file change: {} (size: {})", recorded.hash, recorded.size_bytes);

    // Output and verify
    let output_dir = setup.work_dir("output");
    let outputter = WorkingDirOutput::new(pristine, change_dir, output_dir.clone());
    outputter.output("main").unwrap();

    assert!(output_dir.join("empty.txt").exists());
    let content = std::fs::read_to_string(output_dir.join("empty.txt")).unwrap();
    assert!(content.is_empty(), "empty file should be empty");
}

/// Test: Binary file handling with all byte values.
#[tokio::test]
async fn test_pijul_real_changes_binary_file_handling() {
    let setup = RealChangesTestSetup::new("binary-file");
    let work_dir = setup.work_dir("work");

    // Create binary file with all byte values 0x00-0xFF
    let binary_data: Vec<u8> = (0..=255).collect();
    std::fs::write(work_dir.join("binary.bin"), &binary_data).unwrap();

    let pristine = setup.pristine_mgr.open_or_create(&setup.repo_id).unwrap();
    let change_dir = setup.change_dir();
    let recorder = ChangeRecorder::new(pristine.clone(), change_dir.clone(), work_dir);

    let result = recorder.record("main", "Add binary file", "Test <test@test.com>").await.unwrap();
    assert!(result.is_some());
    let recorded = result.unwrap();

    println!("Binary file change: {} bytes in change", recorded.size_bytes);

    // Output and verify byte-for-byte
    let output_dir = setup.work_dir("output");
    let outputter = WorkingDirOutput::new(pristine, change_dir, output_dir.clone());
    outputter.output("main").unwrap();

    let output_data = std::fs::read(output_dir.join("binary.bin")).unwrap();
    assert_eq!(output_data, binary_data, "binary content must match exactly");
}

/// Test: Unicode filename handling.
#[tokio::test]
async fn test_pijul_real_changes_unicode_filename_handling() {
    let setup = RealChangesTestSetup::new("unicode-filename");
    let work_dir = setup.work_dir("work");

    // Create files with unicode names
    std::fs::write(work_dir.join("hello_世界.txt"), "Hello World in Chinese\n").unwrap();
    std::fs::write(work_dir.join("привет.txt"), "Hello in Russian\n").unwrap();
    std::fs::write(work_dir.join("🎉_emoji.txt"), "Emoji filename!\n").unwrap();

    let pristine = setup.pristine_mgr.open_or_create(&setup.repo_id).unwrap();
    let change_dir = setup.change_dir();
    let recorder = ChangeRecorder::new(pristine.clone(), change_dir.clone(), work_dir);

    let result = recorder.record("main", "Add unicode files", "Test <test@test.com>").await.unwrap();
    assert!(result.is_some());

    // Output and verify
    let output_dir = setup.work_dir("output");
    let outputter = WorkingDirOutput::new(pristine, change_dir, output_dir.clone());
    outputter.output("main").unwrap();

    assert!(output_dir.join("hello_世界.txt").exists());
    assert!(output_dir.join("привет.txt").exists());
    assert!(output_dir.join("🎉_emoji.txt").exists());

    let content = std::fs::read_to_string(output_dir.join("hello_世界.txt")).unwrap();
    assert_eq!(content, "Hello World in Chinese\n");
}

/// Test: Special characters in file content.
#[tokio::test]
async fn test_pijul_real_changes_special_characters_in_content() {
    let setup = RealChangesTestSetup::new("special-content");
    let work_dir = setup.work_dir("work");

    // Create file with various special characters
    let content = r#"
Special characters test:
- Null: (not representable in text)
- Tab:	<-- tab here
- Newlines: above and below
- Quotes: "double" and 'single'
- Backslashes: \ and \\
- Unicode: 日本語 العربية עברית
- Math: ∑∫∂∇ ≤≥≠ ∈∉
- Currency: $€£¥₿
- Arrows: ←→↑↓ ⇐⇒⇑⇓
"#;
    std::fs::write(work_dir.join("special.txt"), content).unwrap();

    let pristine = setup.pristine_mgr.open_or_create(&setup.repo_id).unwrap();
    let change_dir = setup.change_dir();
    let recorder = ChangeRecorder::new(pristine.clone(), change_dir.clone(), work_dir);

    let result = recorder.record("main", "Add special chars", "Test <test@test.com>").await.unwrap();
    assert!(result.is_some());

    // Output and verify exact content
    let output_dir = setup.work_dir("output");
    let outputter = WorkingDirOutput::new(pristine, change_dir, output_dir.clone());
    outputter.output("main").unwrap();

    let output_content = std::fs::read_to_string(output_dir.join("special.txt")).unwrap();
    assert_eq!(output_content, content, "special characters must be preserved");
}

/// Test: Moderately large file (1MB - not near limit but still substantial).
#[tokio::test]
async fn test_pijul_real_changes_moderately_large_file() {
    let setup = RealChangesTestSetup::new("large-file");
    let work_dir = setup.work_dir("work");

    // Create 1MB file with pattern
    let line = "This is a test line for large file handling.\n";
    let repeat_count = 1024 * 1024 / line.len();
    let content: String = line.repeat(repeat_count);
    std::fs::write(work_dir.join("large.txt"), &content).unwrap();

    println!("Created {}MB file", content.len() / (1024 * 1024));

    let pristine = setup.pristine_mgr.open_or_create(&setup.repo_id).unwrap();
    let change_dir = setup.change_dir();
    let recorder = ChangeRecorder::new(pristine.clone(), change_dir.clone(), work_dir);

    let result = recorder.record("main", "Add large file", "Test <test@test.com>").await.unwrap();
    assert!(result.is_some());
    let recorded = result.unwrap();

    println!("Large file change: {} bytes (compressed: {})", content.len(), recorded.size_bytes);

    // Output and verify
    let output_dir = setup.work_dir("output");
    let outputter = WorkingDirOutput::new(pristine, change_dir, output_dir.clone());
    outputter.output("main").unwrap();

    let output_content = std::fs::read_to_string(output_dir.join("large.txt")).unwrap();
    assert_eq!(output_content.len(), content.len(), "large file size must match");
    assert_eq!(output_content, content, "large file content must match");
}

/// Test: Apply change to fresh pristine (simulating sync to new node).
#[tokio::test]
async fn test_pijul_real_changes_apply_to_fresh_pristine() {
    let setup = RealChangesTestSetup::new("apply-fresh");
    let work_dir = setup.work_dir("work");

    // Create a file and record
    std::fs::write(work_dir.join("sync.txt"), "Sync test content\n").unwrap();

    let pristine_a = setup.pristine_mgr.open_or_create(&setup.repo_id).unwrap();
    let change_dir = setup.change_dir();
    let recorder = ChangeRecorder::new(pristine_a, change_dir.clone(), work_dir);

    let recorded = recorder.record("main", "Initial", "Test <test@test.com>").await.unwrap().unwrap();

    println!("Source change: {}", recorded.hash);

    // Create a fresh pristine (simulating a new node)
    let fresh_repo_id = RepoId::from_hash(blake3::hash(b"fresh-node-repo"));
    let pristine_b = setup.pristine_mgr.open_or_create(&fresh_repo_id).unwrap();

    // Create change directory for the fresh repo sharing the same blob store
    let change_dir_b = ChangeDirectory::new(&setup.data_dir, fresh_repo_id, setup.change_store.clone());
    change_dir_b.ensure_dir().unwrap();

    // Copy the change (simulates P2P sync)
    let change_bytes = setup.change_store.get_change(&recorded.hash).await.unwrap().expect("change exists");
    change_dir_b.store_change(&change_bytes).await.unwrap();

    // Apply to fresh pristine
    let applicator = ChangeApplicator::new(pristine_b.clone(), change_dir_b.clone());
    let apply_result = applicator.apply_local("main", &recorded.hash).unwrap();

    println!("Applied {} changes to fresh pristine", apply_result.changes_applied);

    // Output and verify
    let output_dir = setup.work_dir("output_b");
    let outputter = WorkingDirOutput::new(pristine_b, change_dir_b, output_dir.clone());
    let output_result = outputter.output("main").unwrap();

    assert!(output_result.is_clean());

    let content = std::fs::read_to_string(output_dir.join("sync.txt")).unwrap();
    assert_eq!(content, "Sync test content\n");
}
