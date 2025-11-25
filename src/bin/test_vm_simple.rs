// Simple test to start a Cloud Hypervisor VM
use anyhow::Result;
use mvm_ci::vm_manager::{VmManager, VmManagerConfig};
use mvm_ci::vm_manager::vm_types::{VmConfig, VmMode, IsolationLevel};
use mvm_ci::hiqlite_service::HiqliteService;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info,mvm_ci=debug")
        .init();

    println!("=== Simple VM Start Test ===\n");

    // Set required environment variables
    unsafe {
        std::env::set_var("HQL_NODE_ID", "1");
        std::env::set_var("HQL_SECRET_RAFT", "test-secret-raft-1234567890");
        std::env::set_var("HQL_SECRET_API", "test-secret-api-1234567890");
    }

    // Create a test Hiqlite service
    println!("Initializing Hiqlite service...");
    let hiqlite = Arc::new(
        HiqliteService::new(Some(PathBuf::from("/tmp/mvm-ci-test/hiqlite")))
            .await?
    );
    println!("✓ Hiqlite service started");

    // Initialize the database schema
    println!("Initializing database schema...");
    hiqlite.initialize_schema().await?;
    println!("✓ Database schema initialized\n");

    // Configure VM Manager
    let config = VmManagerConfig {
        max_vms: 3,
        auto_scaling: false,
        pre_warm_count: 0,
        flake_dir: PathBuf::from("./microvms"),
        state_dir: PathBuf::from("/tmp/mvm-ci-test/vms"),
        default_memory_mb: 512,
        default_vcpus: 1,
    };

    println!("Creating VM Manager...");
    println!("  Max VMs: {}", config.max_vms);
    println!("  State directory: {:?}", config.state_dir);
    println!("  Flake directory: {:?}", config.flake_dir);
    println!();

    // Create VM Manager
    let vm_manager = VmManager::new(config, hiqlite).await?;
    println!("✓ VM Manager initialized\n");

    // Start a simple ephemeral VM
    println!("Starting ephemeral VM...");
    let vm_config = VmConfig {
        id: Uuid::new_v4(),
        mode: VmMode::Ephemeral {
            job_id: "test-job-1".to_string(),
        },
        isolation_level: IsolationLevel::Standard,
        memory_mb: 512,
        vcpus: 1,
        hypervisor: "cloud-hypervisor".to_string(),
        capabilities: vec![],
    };
    let vm = vm_manager.start_vm(vm_config).await?;

    println!("✓ VM Started!");
    println!("  VM ID: {}", vm.config.id);
    println!("  State: {:?}", vm.state);
    println!("  IP Address: {:?}", vm.ip_address);
    println!("  PID: {:?}", vm.pid);
    println!();

    // Wait a bit to see it running
    println!("VM running... (waiting 10 seconds)");
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

    // Stop the VM
    println!("Stopping VM...");
    vm_manager.stop_vm(vm.config.id).await?;
    println!("✓ VM Stopped\n");

    println!("Test completed successfully!");
    Ok(())
}