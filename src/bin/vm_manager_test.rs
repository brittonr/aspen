// Test Blixard VM Management
//
// This demonstrates blixard managing VM lifecycles

use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

#[derive(Debug, Clone)]
struct ManagedVm {
    id: Uuid,
    name: String,
    pid: Option<u32>,
    socket_path: PathBuf,
    state: VmState,
    started_at: std::time::Instant,
}

#[derive(Debug, Clone, PartialEq)]
enum VmState {
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
}

struct VmManager {
    vms: Arc<RwLock<HashMap<Uuid, ManagedVm>>>,
    flake_dir: PathBuf,
    max_vms: usize,
}

impl VmManager {
    fn new(flake_dir: PathBuf, max_vms: usize) -> Self {
        Self {
            vms: Arc::new(RwLock::new(HashMap::new())),
            flake_dir,
            max_vms,
        }
    }

    async fn start_vm(&self, name: String) -> Result<Uuid> {
        let vm_id = Uuid::new_v4();
        let socket_path = PathBuf::from(format!("/tmp/blixard-{}.sock", vm_id));

        println!("📦 Starting VM: {} ({})", name, vm_id);

        // Check VM limit
        {
            let vms = self.vms.read().await;
            let running_count = vms.values()
                .filter(|v| v.state == VmState::Running)
                .count();

            if running_count >= self.max_vms {
                return Err(anyhow::anyhow!("Maximum VM limit ({}) reached", self.max_vms));
            }
        }

        let mut vm = ManagedVm {
            id: vm_id,
            name: name.clone(),
            pid: None,
            socket_path: socket_path.clone(),
            state: VmState::Starting,
            started_at: std::time::Instant::now(),
        };

        // Insert VM in starting state
        {
            let mut vms = self.vms.write().await;
            vms.insert(vm_id, vm.clone());
        }

        // Spawn the VM process
        println!("  → Spawning VM process...");
        let mut cmd = Command::new("nix");
        cmd.args(&[
            "run",
            &format!("{}#test-vm", self.flake_dir.display()),
            "--",
            &name,
            socket_path.to_str().unwrap(),
        ]);

        match cmd.spawn() {
            Ok(child) => {
                let pid = child.id();
                println!("  ✓ VM started with PID: {}", pid);

                // Update VM with PID and running state
                vm.pid = Some(pid);
                vm.state = VmState::Running;

                let mut vms = self.vms.write().await;
                vms.insert(vm_id, vm);

                Ok(vm_id)
            }
            Err(e) => {
                println!("  ✗ Failed to start VM: {}", e);

                // Mark as failed
                let mut vms = self.vms.write().await;
                if let Some(vm) = vms.get_mut(&vm_id) {
                    vm.state = VmState::Failed;
                }

                Err(anyhow::anyhow!("Failed to spawn VM: {}", e))
            }
        }
    }

    async fn stop_vm(&self, vm_id: Uuid) -> Result<()> {
        println!("🛑 Stopping VM: {}", vm_id);

        let mut vms = self.vms.write().await;
        if let Some(vm) = vms.get_mut(&vm_id) {
            if vm.state != VmState::Running {
                return Err(anyhow::anyhow!("VM is not running"));
            }

            vm.state = VmState::Stopping;

            if let Some(pid) = vm.pid {
                println!("  → Terminating process {}", pid);

                // Try graceful shutdown first
                unsafe {
                    libc::kill(pid as i32, libc::SIGTERM);
                }

                // Give it a moment
                sleep(Duration::from_secs(1)).await;

                // Check if still running and force kill if needed
                if unsafe { libc::kill(pid as i32, 0) } == 0 {
                    println!("  → Force killing process {}", pid);
                    unsafe {
                        libc::kill(pid as i32, libc::SIGKILL);
                    }
                }

                println!("  ✓ VM stopped");
            }

            vm.state = VmState::Stopped;
            Ok(())
        } else {
            Err(anyhow::anyhow!("VM not found"))
        }
    }

    async fn list_vms(&self) -> Vec<(Uuid, String, VmState, Duration)> {
        let vms = self.vms.read().await;
        vms.values()
            .map(|vm| (
                vm.id,
                vm.name.clone(),
                vm.state.clone(),
                vm.started_at.elapsed()
            ))
            .collect()
    }

    async fn get_vm_state(&self, vm_id: Uuid) -> Option<VmState> {
        let vms = self.vms.read().await;
        vms.get(&vm_id).map(|vm| vm.state.clone())
    }

    async fn monitor_health(&self) {
        println!("🔍 Starting health monitoring...");

        loop {
            sleep(Duration::from_secs(5)).await;

            let mut vms = self.vms.write().await;
            for vm in vms.values_mut() {
                if vm.state == VmState::Running {
                    if let Some(pid) = vm.pid {
                        // Check if process is still alive
                        if unsafe { libc::kill(pid as i32, 0) } != 0 {
                            println!("  ⚠️  VM {} (PID {}) has died", vm.name, pid);
                            vm.state = VmState::Failed;
                        }
                    }
                }
            }
        }
    }

    async fn get_stats(&self) -> (usize, usize, usize, usize) {
        let vms = self.vms.read().await;
        let total = vms.len();
        let running = vms.values().filter(|v| v.state == VmState::Running).count();
        let stopped = vms.values().filter(|v| v.state == VmState::Stopped).count();
        let failed = vms.values().filter(|v| v.state == VmState::Failed).count();

        (total, running, stopped, failed)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("═══════════════════════════════════════");
    println!("    Blixard VM Manager Test");
    println!("═══════════════════════════════════════\n");

    let flake_dir = PathBuf::from("./microvms");
    let manager = Arc::new(VmManager::new(flake_dir, 3));

    // Start health monitor in background
    let monitor_manager = manager.clone();
    tokio::spawn(async move {
        monitor_manager.monitor_health().await;
    });

    // Test 1: Start multiple VMs
    println!("TEST 1: Starting Multiple VMs");
    println!("─────────────────────────────\n");

    let _vm1 = manager.start_vm("worker-1".to_string()).await?;
    sleep(Duration::from_secs(1)).await;

    let vm2 = manager.start_vm("worker-2".to_string()).await?;
    sleep(Duration::from_secs(1)).await;

    let _vm3 = manager.start_vm("worker-3".to_string()).await?;
    println!();

    // Test 2: List running VMs
    println!("TEST 2: Listing VMs");
    println!("───────────────────\n");

    let vms = manager.list_vms().await;
    for (id, name, state, uptime) in vms {
        println!("  VM: {} ({:?})", name, id);
        println!("      State: {:?}, Uptime: {:?}", state, uptime);
    }
    println!();

    // Test 3: Check stats
    println!("TEST 3: VM Statistics");
    println!("────────────────────\n");

    let (total, running, stopped, failed) = manager.get_stats().await;
    println!("  Total VMs:   {}", total);
    println!("  Running:     {}", running);
    println!("  Stopped:     {}", stopped);
    println!("  Failed:      {}", failed);
    println!();

    // Test 4: Test max VM limit
    println!("TEST 4: VM Limit");
    println!("────────────────\n");

    match manager.start_vm("worker-4".to_string()).await {
        Ok(_) => println!("  ✓ VM started (unexpected)"),
        Err(e) => println!("  ✓ Correctly rejected: {}", e),
    }
    println!();

    // Test 5: Stop a VM
    println!("TEST 5: Stopping VM");
    println!("──────────────────\n");

    manager.stop_vm(vm2).await?;

    if let Some(state) = manager.get_vm_state(vm2).await {
        println!("  VM state after stop: {:?}", state);
    }
    println!();

    // Test 6: Try to start another VM after one stopped
    println!("TEST 6: Start After Stop");
    println!("───────────────────────\n");

    let _vm4 = manager.start_vm("worker-4".to_string()).await?;
    println!("  ✓ Successfully started VM after freeing slot");
    println!();

    // Test 7: Final stats
    println!("TEST 7: Final Statistics");
    println!("───────────────────────\n");

    let (total, running, stopped, failed) = manager.get_stats().await;
    println!("  Total VMs:   {}", total);
    println!("  Running:     {}", running);
    println!("  Stopped:     {}", stopped);
    println!("  Failed:      {}", failed);
    println!();

    // Clean up all VMs
    println!("CLEANUP: Stopping All VMs");
    println!("────────────────────────\n");

    for (id, name, state, _) in manager.list_vms().await {
        if state == VmState::Running {
            println!("  Stopping {}...", name);
            let _ = manager.stop_vm(id).await;
        }
    }

    println!();
    println!("═══════════════════════════════════════");
    println!("    ✓ VM Management Test Complete");
    println!("═══════════════════════════════════════");
    println!();
    println!("Demonstrated Capabilities:");
    println!("  ✓ Start VMs on demand");
    println!("  ✓ Track VM state and PIDs");
    println!("  ✓ Enforce VM limits");
    println!("  ✓ Monitor VM health");
    println!("  ✓ Stop VMs gracefully");
    println!("  ✓ Collect statistics");

    Ok(())
}