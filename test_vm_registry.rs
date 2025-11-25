// Quick test of VM registry persistence

use mvm_ci::vm_manager::{VmRegistry, VmConfig, VmInstance};
use tempfile::TempDir;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    println!("Testing VM Registry Persistence...\n");

    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path();
    println!("Using temp directory: {:?}", path);

    let vm_id = Uuid::new_v4();
    println!("Generated VM ID: {}", vm_id);

    // Test 1: Create and register VM
    {
        println!("\nPhase 1: Creating registry and registering VM...");
        let registry = VmRegistry::new(path).await.unwrap();

        let config = VmConfig::default_service();
        let mut vm = VmInstance::new(config);
        vm.config.id = vm_id;

        registry.register(vm).await.unwrap();
        println!("VM registered successfully");

        let count = registry.count_all().await;
        println!("VMs in registry: {}", count);
        assert_eq!(count, 1);
    }

    // Test 2: Recover from persistence
    {
        println!("\nPhase 2: Creating new registry and recovering...");
        let registry = VmRegistry::new(path).await.unwrap();

        let recovered = registry.recover_from_persistence().await.unwrap();
        println!("Recovered {} VMs from persistence", recovered);
        assert_eq!(recovered, 1);

        let vm = registry.get(vm_id).await.unwrap();
        assert!(vm.is_some());
        println!("VM {} successfully recovered!", vm_id);
    }

    println!("\n✅ VM Registry persistence test PASSED!");
}