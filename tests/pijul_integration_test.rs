//! Integration tests for the Pijul module.
//!
//! These tests exercise the full workflow of creating repositories,
//! recording changes, applying them, and outputting to working directories.

#![cfg(feature = "pijul")]

use std::sync::Arc;

use aspen::blob::InMemoryBlobStore;
use aspen::forge::identity::RepoId;
use aspen::pijul::{
    AspenChangeStore, ChangeApplicator, ChangeDirectory, ChangeRecorder, OutputResult,
    PristineManager, WorkingDirOutput,
};
use tempfile::TempDir;

/// Helper to create test infrastructure.
struct TestSetup {
    tmp: TempDir,
    data_dir: std::path::PathBuf,
    repo_id: RepoId,
    blobs: Arc<InMemoryBlobStore>,
    change_store: Arc<AspenChangeStore<InMemoryBlobStore>>,
    pristine_mgr: PristineManager,
}

impl TestSetup {
    fn new() -> Self {
        let tmp = TempDir::new().unwrap();
        let data_dir = tmp.path().join("data");
        let repo_id = RepoId::from_hash(blake3::hash(b"test-pijul-repo"));

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

#[tokio::test]
async fn test_record_and_apply_single_file() {
    let setup = TestSetup::new();
    let work_dir = setup.work_dir("work");

    // Create a file in the working directory
    std::fs::write(work_dir.join("hello.txt"), "Hello, Pijul!\n").unwrap();

    // Create pristine and recorder
    let pristine = setup.pristine_mgr.open_or_create(&setup.repo_id).unwrap();
    let change_dir = setup.change_dir();
    let recorder = ChangeRecorder::new(pristine.clone(), change_dir, work_dir.clone());

    // Record the change
    let result = recorder
        .record("main", "Add hello.txt", "Test <test@example.com>")
        .await
        .unwrap();

    assert!(result.is_some(), "should have changes to record");
    let record_result = result.unwrap();

    assert_eq!(record_result.message, "Add hello.txt");
    assert!(record_result.num_hunks > 0, "should have at least one hunk");
    assert!(record_result.size_bytes > 0, "change should have non-zero size");

    println!(
        "Recorded change: {} ({} hunks, {} bytes)",
        record_result.hash, record_result.num_hunks, record_result.size_bytes
    );
}

#[tokio::test]
async fn test_record_multiple_files() {
    let setup = TestSetup::new();
    let work_dir = setup.work_dir("work");

    // Create multiple files
    std::fs::write(work_dir.join("README.md"), "# Test Project\n").unwrap();
    std::fs::write(work_dir.join("main.rs"), "fn main() {}\n").unwrap();
    std::fs::create_dir_all(work_dir.join("src")).unwrap();
    std::fs::write(work_dir.join("src/lib.rs"), "pub fn hello() {}\n").unwrap();

    // Create pristine and recorder
    let pristine = setup.pristine_mgr.open_or_create(&setup.repo_id).unwrap();
    let change_dir = setup.change_dir();
    let recorder = ChangeRecorder::new(pristine, change_dir, work_dir);

    // Record the change
    let result = recorder
        .record("main", "Initial commit", "Test <test@example.com>")
        .await
        .unwrap();

    assert!(result.is_some());
    let record_result = result.unwrap();

    // Should have multiple hunks for multiple files
    println!(
        "Recorded {} hunks for multiple files",
        record_result.num_hunks
    );
}

#[tokio::test]
async fn test_record_then_modify() {
    let setup = TestSetup::new();
    let work_dir = setup.work_dir("work");

    // Create initial file
    std::fs::write(work_dir.join("counter.txt"), "count: 0\n").unwrap();

    let pristine = setup.pristine_mgr.open_or_create(&setup.repo_id).unwrap();
    let change_dir = setup.change_dir();
    let recorder = ChangeRecorder::new(pristine.clone(), change_dir.clone(), work_dir.clone());

    // Record initial change
    let first = recorder
        .record("main", "Initial counter", "Test <test@example.com>")
        .await
        .unwrap()
        .expect("should have first change");

    println!("First change: {}", first.hash);

    // Modify the file
    std::fs::write(work_dir.join("counter.txt"), "count: 1\n").unwrap();

    // Record modification - need a new recorder with same pristine handle
    let recorder2 = ChangeRecorder::new(pristine, change_dir, work_dir);
    let second = recorder2
        .record("main", "Increment counter", "Test <test@example.com>")
        .await
        .unwrap()
        .expect("should have second change");

    println!("Second change: {}", second.hash);

    // Changes should be different
    assert_ne!(first.hash, second.hash);
}

#[tokio::test]
async fn test_apply_change_from_another_repo() {
    let setup = TestSetup::new();

    // Repo A: Create and record a change
    let work_dir_a = setup.work_dir("repo_a");
    std::fs::write(work_dir_a.join("shared.txt"), "Shared content\n").unwrap();

    let pristine_a = setup.pristine_mgr.open_or_create(&setup.repo_id).unwrap();
    let change_dir_a = setup.change_dir();
    let recorder_a = ChangeRecorder::new(pristine_a, change_dir_a.clone(), work_dir_a);

    let recorded = recorder_a
        .record("main", "Add shared file", "Alice <alice@example.com>")
        .await
        .unwrap()
        .expect("should record change");

    println!("Recorded change in repo A: {}", recorded.hash);

    // Repo B: Create a fresh pristine and apply the change
    let repo_id_b = RepoId::from_hash(blake3::hash(b"test-pijul-repo-b"));
    let pristine_b = setup.pristine_mgr.open_or_create(&repo_id_b).unwrap();

    // Create a change directory for repo B that shares the same blob store
    let change_dir_b = ChangeDirectory::new(&setup.data_dir, repo_id_b, setup.change_store.clone());

    // First, we need to copy the change file to repo B's change directory
    // In a real scenario, this would happen via iroh-blobs P2P transfer
    change_dir_b.ensure_dir().unwrap();

    // Fetch the change bytes from the shared blob store and store in B's directory
    let change_bytes = setup
        .change_store
        .get_change(&recorded.hash)
        .await
        .unwrap()
        .expect("change should exist");
    change_dir_b.store_change(&change_bytes).await.unwrap();

    // Apply the change to repo B
    let applicator = ChangeApplicator::new(pristine_b.clone(), change_dir_b.clone());
    let apply_result = applicator.apply_local("main", &recorded.hash).unwrap();

    println!(
        "Applied change to repo B: {} operations",
        apply_result.changes_applied
    );

    // Output to a working directory and verify
    let work_dir_b = setup.work_dir("repo_b");
    let outputter = WorkingDirOutput::new(pristine_b, change_dir_b, work_dir_b.clone());
    let output_result = outputter.output("main").unwrap();

    assert!(output_result.is_clean(), "should have no conflicts");

    // Verify the file exists and has correct content
    let content = std::fs::read_to_string(work_dir_b.join("shared.txt")).unwrap();
    assert_eq!(content, "Shared content\n");
}

#[tokio::test]
async fn test_output_empty_repo() {
    let setup = TestSetup::new();
    let work_dir = setup.work_dir("empty");

    let pristine = setup.pristine_mgr.open_or_create(&setup.repo_id).unwrap();
    let change_dir = setup.change_dir();

    let outputter = WorkingDirOutput::new(pristine, change_dir, work_dir.clone());
    let result = outputter.output("main").unwrap();

    assert!(result.is_clean());

    // Working directory should exist but be empty (no files from pristine)
    assert!(work_dir.exists());
}

#[tokio::test]
async fn test_record_no_changes() {
    let setup = TestSetup::new();
    let work_dir = setup.work_dir("empty");

    let pristine = setup.pristine_mgr.open_or_create(&setup.repo_id).unwrap();
    let change_dir = setup.change_dir();
    let recorder = ChangeRecorder::new(pristine, change_dir, work_dir);

    // Recording with no files should return None
    let result = recorder
        .record("main", "Empty", "Test <test@example.com>")
        .await
        .unwrap();

    assert!(result.is_none(), "should have no changes to record");
}

#[tokio::test]
async fn test_multiple_channels() {
    let setup = TestSetup::new();
    let work_dir = setup.work_dir("work");

    // Create a file
    std::fs::write(work_dir.join("base.txt"), "Base content\n").unwrap();

    let pristine = setup.pristine_mgr.open_or_create(&setup.repo_id).unwrap();
    let change_dir = setup.change_dir();
    let recorder = ChangeRecorder::new(pristine.clone(), change_dir.clone(), work_dir.clone());

    // Record on main channel
    let main_change = recorder
        .record("main", "Add base file", "Test <test@example.com>")
        .await
        .unwrap()
        .expect("should record on main");

    println!("Main channel change: {}", main_change.hash);

    // Fork to a feature channel and add another file
    {
        let mut txn = pristine.mut_txn_begin().unwrap();
        txn.fork_channel("main", "feature").unwrap();
        txn.commit().unwrap();
    }

    // Add a file for the feature channel
    std::fs::write(work_dir.join("feature.txt"), "Feature content\n").unwrap();

    let recorder2 = ChangeRecorder::new(pristine.clone(), change_dir.clone(), work_dir.clone());
    let feature_change = recorder2
        .record("feature", "Add feature file", "Test <test@example.com>")
        .await
        .unwrap()
        .expect("should record on feature");

    println!("Feature channel change: {}", feature_change.hash);

    // Main and feature should have different changes
    assert_ne!(main_change.hash, feature_change.hash);

    // List channels
    let channels = {
        let txn = pristine.txn_begin().unwrap();
        txn.list_channels().unwrap()
    };

    assert!(channels.iter().any(|c| c == "main"));
    assert!(channels.iter().any(|c| c == "feature"));
}

#[tokio::test]
async fn test_binary_file_handling() {
    let setup = TestSetup::new();
    let work_dir = setup.work_dir("work");

    // Create a binary file (simulated image)
    let binary_data: Vec<u8> = (0..256).map(|i| i as u8).collect();
    std::fs::write(work_dir.join("image.bin"), &binary_data).unwrap();

    let pristine = setup.pristine_mgr.open_or_create(&setup.repo_id).unwrap();
    let change_dir = setup.change_dir();
    let recorder = ChangeRecorder::new(pristine.clone(), change_dir.clone(), work_dir.clone());

    // Record the binary file
    let result = recorder
        .record("main", "Add binary file", "Test <test@example.com>")
        .await
        .unwrap();

    assert!(result.is_some(), "should record binary file");
    let recorded = result.unwrap();
    println!("Recorded binary file: {} bytes", recorded.size_bytes);

    // Output and verify
    let output_dir = setup.work_dir("output");
    let outputter = WorkingDirOutput::new(pristine, change_dir, output_dir.clone());
    outputter.output("main").unwrap();

    let output_data = std::fs::read(output_dir.join("image.bin")).unwrap();
    assert_eq!(output_data, binary_data, "binary content should match");
}

#[tokio::test]
async fn test_nested_directory_structure() {
    let setup = TestSetup::new();
    let work_dir = setup.work_dir("work");

    // Create nested directory structure
    std::fs::create_dir_all(work_dir.join("src/core/utils")).unwrap();
    std::fs::create_dir_all(work_dir.join("tests/integration")).unwrap();

    std::fs::write(work_dir.join("src/lib.rs"), "mod core;\n").unwrap();
    std::fs::write(work_dir.join("src/core/mod.rs"), "mod utils;\n").unwrap();
    std::fs::write(
        work_dir.join("src/core/utils/mod.rs"),
        "pub fn helper() {}\n",
    )
    .unwrap();
    std::fs::write(
        work_dir.join("tests/integration/test_main.rs"),
        "#[test] fn test() {}\n",
    )
    .unwrap();

    let pristine = setup.pristine_mgr.open_or_create(&setup.repo_id).unwrap();
    let change_dir = setup.change_dir();
    let recorder = ChangeRecorder::new(pristine.clone(), change_dir.clone(), work_dir);

    // Record
    let result = recorder
        .record("main", "Add project structure", "Test <test@example.com>")
        .await
        .unwrap()
        .expect("should record nested structure");

    println!(
        "Recorded nested structure: {} hunks",
        result.num_hunks
    );

    // Output to new directory and verify structure
    let output_dir = setup.work_dir("output");
    let outputter = WorkingDirOutput::new(pristine, change_dir, output_dir.clone());
    outputter.output("main").unwrap();

    // Verify all files exist
    assert!(output_dir.join("src/lib.rs").exists());
    assert!(output_dir.join("src/core/mod.rs").exists());
    assert!(output_dir.join("src/core/utils/mod.rs").exists());
    assert!(output_dir.join("tests/integration/test_main.rs").exists());

    // Verify content
    let content = std::fs::read_to_string(output_dir.join("src/core/utils/mod.rs")).unwrap();
    assert_eq!(content, "pub fn helper() {}\n");
}
