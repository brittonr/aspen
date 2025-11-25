# VM Management System Usage Guide

## Overview
The Blixard VM Management system orchestrates Cloud Hypervisor microvms with distributed state management via Hiqlite.

## Starting the System

### 1. Start the Control Plane
```bash
# Start the main application (includes VM Manager)
cargo run --bin mvm-ci

# The VM Manager will:
# - Initialize VM tables in Hiqlite
# - Start health monitoring
# - Pre-warm VMs if configured
# - Recover existing VMs from Hiqlite
```

### 2. Environment Variables
```bash
# VM Configuration
export FIRECRACKER_MAX_CONCURRENT_VMS=10    # Max VMs allowed
export FIRECRACKER_DEFAULT_MEMORY_MB=512    # Default RAM per VM
export FIRECRACKER_DEFAULT_VCPUS=2          # Default vCPUs
export FIRECRACKER_FLAKE_DIR=./microvms     # Nix flake location
export VM_STATE_DIR=./data/vm-state         # VM state directory
```

## Using VMs Through the Application

### Direct VM Management (via code)

```rust
// Access VM Manager from app state
let vm_manager = state.infrastructure().vm_manager();

// Start a new VM
let config = VmConfig::default_ephemeral();
let vm_instance = vm_manager.start_vm(config).await?;

// Submit job to specific VM
let job = Job { /* ... */ };
let result = vm_manager.submit_job_to_vm(vm_instance.config.id, job).await?;

// Stop a VM
vm_manager.stop_vm(vm_instance.config.id).await?;

// Get VM statistics
let stats = vm_manager.get_stats().await?;
println!("Total VMs: {}, Running: {}", stats.total_vms, stats.running_vms);
```

### Job Routing (automatic VM assignment)

```rust
// Submit job - VM Manager will route to best VM
let job = Job {
    id: Uuid::new_v4(),
    job_type: JobType::Execute,
    // ... job details
};

let result = vm_manager.execute_job(job).await?;
// Returns VmAssignment::Ephemeral(vm_id) or VmAssignment::Service(vm_id)
```

## VM Lifecycle States

```
Starting → Ready → Busy → Idle → Draining → Terminated
                     ↓
                  Failed
```

- **Starting**: VM is booting
- **Ready**: VM is ready to accept jobs
- **Busy**: VM is executing a job
- **Idle**: Service VM waiting for next job
- **Draining**: VM is shutting down gracefully
- **Terminated**: VM has stopped
- **Failed**: VM encountered an error

## Monitoring VMs

### Via Hiqlite SQL
```sql
-- List all VMs
SELECT id, state, node_id, created_at FROM vms;

-- Count VMs by state
SELECT state, COUNT(*) FROM vms GROUP BY state;

-- View VM events
SELECT * FROM vm_events WHERE vm_id = ? ORDER BY timestamp DESC;

-- Find ready VMs
SELECT id, config FROM vms WHERE state = 'Ready';
```

### Via Application Logs
```bash
# Watch VM Manager logs
cargo run --bin mvm-ci 2>&1 | grep -E "VM|vm"

# Example output:
# INFO Starting VM Manager...
# INFO VM registered vm_id=abc-123 state=Starting
# INFO VM state changed vm_id=abc-123 Starting -> Ready
# INFO Job assigned to VM job_id=xyz-456 vm_id=abc-123
```

## Testing VM Integration

### Manual Test
```bash
# 1. Start a test VM
cd microvms
nix run .#run-service-vm

# 2. In another terminal, check VM is tracked in Hiqlite
sqlite3 ./data/hiqlite/hiqlite.db "SELECT * FROM vms;"
```

### Programmatic Test
See `test-vm-hiqlite.sh` for automated testing.

## VM Types

### Ephemeral VMs
- One job then terminate
- Minimal resources
- Clean environment per job
- Good for: untrusted code, isolation

### Service VMs
- Long-running, multiple jobs
- More resources
- Reused across jobs
- Good for: trusted workloads, performance

## Configuration

In `src/config.rs`:
- `FirecrackerConfig`: VM hypervisor settings
- `StorageConfig.vm_state_dir`: Where VM state is stored

In VM Manager:
- `max_vms`: Maximum concurrent VMs
- `auto_scaling`: Enable/disable auto-scaling
- `pre_warm_count`: Pre-warmed idle VMs
- `default_memory_mb`: Default RAM allocation
- `default_vcpus`: Default CPU allocation

## Troubleshooting

### VMs Not Starting
```bash
# Check logs
journalctl -f | grep cloud-hypervisor

# Check VM state in Hiqlite
sqlite3 ./data/hiqlite/hiqlite.db "SELECT id, state, node_id FROM vms WHERE state = 'Failed';"

# Clean up stale VMs
sqlite3 ./data/hiqlite/hiqlite.db "DELETE FROM vms WHERE state = 'Failed';"
```

### Permission Issues
```bash
# Cloud Hypervisor needs permissions for TAP networking
sudo setcap cap_net_admin+ep $(which cloud-hypervisor)
```

### Resource Limits
```bash
# Check current VMs
sqlite3 ./data/hiqlite/hiqlite.db "SELECT COUNT(*) FROM vms WHERE state IN ('Ready', 'Busy');"

# Increase limit
export FIRECRACKER_MAX_CONCURRENT_VMS=20
```

## API Endpoints (To Be Implemented)

Future REST API endpoints:
- `GET /api/vms` - List all VMs
- `POST /api/vms` - Create new VM
- `GET /api/vms/{id}` - Get VM details
- `DELETE /api/vms/{id}` - Stop VM
- `POST /api/vms/{id}/jobs` - Submit job to VM
- `GET /api/vms/stats` - Get VM statistics