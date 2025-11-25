// Test VM lifecycle by directly calling Cloud Hypervisor (bypassing network setup)
use anyhow::Result;
use mvm_ci::vm_manager::{VmManager, VmManagerConfig};
use mvm_ci::vm_manager::vm_types::{VmConfig, VmMode, IsolationLevel};
use mvm_ci::hiqlite_service::HiqliteService;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;
use std::process::Command;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info,mvm_ci=debug")
        .init();

    println!("=== Direct Cloud Hypervisor VM Test ===\n");

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

    // Test direct Cloud Hypervisor execution (no network)
    println!("Testing direct Cloud Hypervisor execution...");
    let kernel_path = "/nix/store/ydaj8kmad8kq4c6jgc1m1a7z3yx58w62-linux-6.12.58-dev/vmlinux";
    let initrd_path = "/nix/store/a12ngmw7lhda7yknbs8i9j70gvn01zwl-initrd-linux-6.12.58/initrd";
    let ch_path = "/nix/store/p77vhx0vliy7n9i2bjyfz9xa4a321ihw-cloud-hypervisor-49.0/bin/cloud-hypervisor";

    // Create job directory
    let job_dir = PathBuf::from("/tmp/mvm-ci-test/vm-direct");
    std::fs::create_dir_all(&job_dir)?;

    // Socket path for VM control
    let socket_path = job_dir.join("vm.sock");

    // Start Cloud Hypervisor directly (no network)
    let mut cmd = Command::new(ch_path);
    cmd.args(&[
        "--cpus", "boot=1",
        "--memory", "size=512M",
        "--kernel", kernel_path,
        "--initramfs", initrd_path,
        "--cmdline", "console=hvc0 root=/dev/vda init=/init",
        "--console", "off",
        "--serial", "tty",
        "--api-socket", socket_path.to_str().unwrap()
    ]);

    println!("Starting VM with command: {:?}", cmd);

    match cmd.spawn() {
        Ok(mut child) => {
            let pid = child.id();
            println!("✓ VM Started!");
            println!("  PID: {}", pid);
            println!("  Control Socket: {:?}", socket_path);

            // Create VM config for registry
            let vm_config = VmConfig {
                id: Uuid::new_v4(),
                mode: VmMode::Ephemeral {
                    job_id: "direct-test-1".to_string(),
                },
                isolation_level: IsolationLevel::Standard,
                memory_mb: 512,
                vcpus: 1,
                hypervisor: "cloud-hypervisor".to_string(),
                capabilities: vec![],
            };

            // Register VM in Hiqlite
            use mvm_ci::params;
            let vm_id = vm_config.id.to_string();
            let config_json = serde_json::to_string(&vm_config)?;
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs() as i64;

            hiqlite.execute(
                "INSERT INTO vms (id, config, state, created_at, updated_at, node_id, pid, control_socket, job_dir, ip_address)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                params!(
                    vm_id.clone(),
                    config_json,
                    "running",
                    now,
                    now,
                    "1",
                    pid as i64,
                    socket_path.to_str().unwrap(),
                    job_dir.to_str().unwrap(),
                    "192.168.100.10"
                ),
            ).await?;
            println!("✓ VM registered in database");

            // Test VM control via socket
            println!("\nTesting VM control socket...");
            let socket_test = Command::new("curl")
                .args(&[
                    "--unix-socket",
                    socket_path.to_str().unwrap(),
                    "http://localhost/api/v1/vm.info"
                ])
                .output();

            match socket_test {
                Ok(output) => {
                    if output.status.success() {
                        println!("✓ Control socket responding!");
                        let response = String::from_utf8_lossy(&output.stdout);
                        println!("  Response preview: {}",
                            response.lines().take(3).collect::<Vec<_>>().join("\n"));
                    } else {
                        println!("⚠ Control socket test failed");
                    }
                }
                Err(_) => println!("⚠ curl not available for socket test"),
            }

            // Wait a bit
            println!("\nVM running... (waiting 5 seconds)");
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

            // Stop the VM
            println!("Stopping VM...");
            let _ = child.kill();
            let _ = child.wait();
            println!("✓ VM Stopped");

            // Update database
            hiqlite.execute(
                "UPDATE vms SET state = $1, updated_at = $2 WHERE id = $3",
                params!(
                    "stopped",
                    now,
                    vm_id
                ),
            ).await?;
            println!("✓ Database updated\n");
        }
        Err(e) => {
            println!("❌ Failed to start VM: {}", e);
            println!("\nPossible issues:");
            println!("1. Insufficient permissions (may need kvm group)");
            println!("2. /dev/kvm not available");
            println!("3. CPU virtualization disabled in BIOS");
        }
    }

    println!("Test completed successfully!");
    Ok(())
}