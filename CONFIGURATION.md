# Blixard Configuration Guide

This document describes the configuration system for Blixard, including TOML file support and environment variable overrides.

## Overview

Blixard uses a layered configuration system that supports multiple configuration sources with clear precedence rules. Configuration can be provided through:

1. TOML configuration files
2. Environment variables
3. Hardcoded defaults

## Configuration Precedence

Configuration values are loaded with the following precedence (highest to lowest):

1. **Environment variables** - Always override all other sources
2. **CONFIG_FILE environment variable** - Path to a custom config file
3. **./config.toml** - Project root configuration file
4. **./config/default.toml** - Default configuration with documentation
5. **Hardcoded defaults** - Built into the application

This means you can:
- Use default.toml as a base configuration
- Override specific values in config.toml
- Override any value with environment variables

## Quick Start

### Development

For development, simply run the application. It will automatically use `./config/default.toml`:

```bash
nix develop -c cargo run
```

### Production

1. Copy the example production configuration:
   ```bash
   cp config/example-production.toml config.toml
   ```

2. Edit config.toml to match your environment

3. Run the application:
   ```bash
   nix develop -c cargo run
   ```

### Custom Configuration File

Use the CONFIG_FILE environment variable to specify a custom location:

```bash
export CONFIG_FILE=/etc/blixard/production.toml
nix develop -c cargo run
```

## Configuration Sections

### Network Configuration

Controls HTTP server and P2P networking:

```toml
[network]
http_port = 3020              # HTTP server port
http_bind_addr = "0.0.0.0"    # Bind address (0.0.0.0 = all interfaces)
iroh_alpn = "iroh+h3"         # Iroh ALPN protocol identifier
```

Environment variables:
- `HTTP_PORT`
- `HTTP_BIND_ADDR`
- `IROH_ALPN`

### Storage Configuration

Defines data storage locations:

```toml
[storage]
iroh_blobs_path = "./data/iroh-blobs"      # Iroh content-addressed storage
hiqlite_data_dir = "./data/hiqlite"        # Distributed SQLite database
vm_state_dir = "./data/vm-state"           # VM metadata and logs
work_dir = "/tmp/tofu-work"                # Temporary working directory
```

Environment variables:
- `IROH_BLOBS_PATH`
- `HQL_DATA_DIR`
- `VM_STATE_DIR`
- `WORK_DIR`

### Flawless WASM Runtime

Configuration for the Flawless WASM execution backend:

```toml
[flawless]
flawless_url = "http://localhost:27288"    # Flawless server URL
```

Environment variables:
- `FLAWLESS_URL`

### Virtual Machine Configuration

Settings for Cloud Hypervisor / Firecracker VMs:

```toml
[vm]
flake_dir = "./microvms"                   # Nix flakes for VM images
state_dir = "./data/firecracker-vms"       # VM state storage
default_memory_mb = 512                    # Default VM memory (MB)
default_vcpus = 1                          # Default vCPU count
max_concurrent_vms = 10                    # Maximum concurrent VMs
auto_scaling = true                        # Enable auto-scaling
pre_warm_count = 2                         # Pre-warmed idle VMs
```

Environment variables:
- `FIRECRACKER_FLAKE_DIR`
- `FIRECRACKER_STATE_DIR`
- `FIRECRACKER_DEFAULT_MEMORY_MB`
- `FIRECRACKER_DEFAULT_VCPUS`
- `FIRECRACKER_MAX_CONCURRENT_VMS`
- `VM_AUTO_SCALING`
- `VM_PRE_WARM_COUNT`

### Timing Configuration

Sleep intervals and retry delays:

```toml
[timing]
worker_no_work_sleep_secs = 2              # Sleep when no work available
worker_error_sleep_secs = 5                # Sleep after errors
worker_heartbeat_interval_secs = 30        # Worker heartbeat interval
hiqlite_startup_delay_secs = 3             # Hiqlite startup delay
hiqlite_check_timeout_secs = 60            # Hiqlite health check timeout
hiqlite_log_throttle_secs = 5              # Log throttling interval
hiqlite_retry_delay_secs = 1               # Retry delay for Hiqlite
```

Environment variables:
- `WORKER_NO_WORK_SLEEP_SECS`
- `WORKER_ERROR_SLEEP_SECS`
- `WORKER_HEARTBEAT_INTERVAL_SECS`
- `HIQLITE_STARTUP_DELAY_SECS`
- `HIQLITE_CHECK_TIMEOUT_SECS`
- `HIQLITE_LOG_THROTTLE_SECS`
- `HIQLITE_RETRY_DELAY_SECS`

### Timeout Configuration

Operation timeouts throughout the system:

```toml
[timeouts]
vm_startup_timeout_secs = 30               # VM boot timeout
vm_shutdown_timeout_secs = 30              # VM shutdown timeout
vm_shutdown_retry_delay_millis = 500       # Shutdown retry delay
vm_health_check_timeout_secs = 5           # VM health check timeout
control_protocol_read_timeout_secs = 5     # Control socket read timeout
control_protocol_shutdown_timeout_secs = 10 # Control shutdown timeout
control_protocol_connect_timeout_secs = 5  # Control connect timeout
iroh_online_timeout_secs = 10              # Iroh endpoint online timeout
adapter_default_timeout_secs = 300         # Default adapter timeout
adapter_wait_timeout_secs = 30             # Adapter wait timeout
adapter_poll_interval_millis = 500         # Adapter polling interval
tofu_plan_timeout_secs = 600               # OpenTofu plan timeout
```

Environment variables:
- `VM_STARTUP_TIMEOUT_SECS`
- `VM_SHUTDOWN_TIMEOUT_SECS`
- `VM_SHUTDOWN_RETRY_DELAY_MILLIS`
- `VM_HEALTH_CHECK_TIMEOUT_SECS`
- `CONTROL_PROTOCOL_READ_TIMEOUT_SECS`
- `CONTROL_PROTOCOL_SHUTDOWN_TIMEOUT_SECS`
- `CONTROL_PROTOCOL_CONNECT_TIMEOUT_SECS`
- `IROH_ONLINE_TIMEOUT_SECS`
- `ADAPTER_DEFAULT_TIMEOUT_SECS`
- `ADAPTER_WAIT_TIMEOUT_SECS`
- `ADAPTER_POLL_INTERVAL_MILLIS`
- `TOFU_PLAN_TIMEOUT_SECS`

### Health Check Configuration

VM and service health monitoring:

```toml
[health_check]
check_interval_secs = 30                   # Health check interval
check_timeout_secs = 5                     # Health check timeout
failure_threshold = 3                      # Failures before unhealthy
recovery_threshold = 2                     # Successes before healthy
enable_circuit_breaker = true              # Enable circuit breaker
circuit_break_duration_secs = 60           # Circuit breaker cooldown
```

Environment variables:
- `HEALTH_CHECK_INTERVAL_SECS`
- `HEALTH_CHECK_TIMEOUT_SECS`
- `HEALTH_FAILURE_THRESHOLD`
- `HEALTH_RECOVERY_THRESHOLD`
- `HEALTH_CIRCUIT_BREAKER_ENABLED`
- `HEALTH_CIRCUIT_BREAK_DURATION_SECS`

### Resource Monitor Configuration

System resource monitoring:

```toml
[resource_monitor]
monitor_interval_secs = 10                 # Monitoring interval
```

Environment variables:
- `RESOURCE_MONITOR_INTERVAL_SECS`

### Adapter Configuration

Job execution adapter settings:

```toml
[adapter]
max_concurrent = 10                        # Max concurrent executions
max_executions = 20                        # Total execution limit
execution_delay_ms = 100                   # Mock adapter delay
max_submission_retries = 3                 # Submission retry limit
health_check_interval_secs = 30            # Adapter health check interval
```

Environment variables:
- `ADAPTER_MAX_CONCURRENT`
- `ADAPTER_MAX_EXECUTIONS`
- `ADAPTER_EXECUTION_DELAY_MS`
- `ADAPTER_MAX_SUBMISSION_RETRIES`
- `ADAPTER_HEALTH_CHECK_INTERVAL_SECS`

### Job Router Configuration

Job routing and VM assignment:

```toml
[job_router]
max_jobs_per_vm = 50                       # Max jobs per VM
auto_create_vms = true                     # Auto-create VMs as needed
prefer_service_vms = true                  # Prefer long-lived VMs
```

Environment variables:
- `JOB_ROUTER_MAX_JOBS_PER_VM`
- `JOB_ROUTER_AUTO_CREATE_VMS`
- `JOB_ROUTER_PREFER_SERVICE_VMS`

### Validation Configuration

Input validation rules:

```toml
[validation]
job_id_min_length = 1                      # Min job ID length
job_id_max_length = 255                    # Max job ID length
payload_max_size_bytes = 1048576           # Max payload size (1MB)
```

Environment variables:
- `VALIDATION_JOB_ID_MIN_LENGTH`
- `VALIDATION_JOB_ID_MAX_LENGTH`
- `VALIDATION_PAYLOAD_MAX_SIZE_BYTES`

### VM Check Configuration

VM readiness checking:

```toml
[vm_check]
initial_interval_secs = 1                  # Initial check interval
max_interval_secs = 10                     # Max check interval (backoff)
timeout_secs = 300                         # Overall check timeout
```

Environment variables:
- `VM_CHECK_INITIAL_INTERVAL_SECS`
- `VM_CHECK_MAX_INTERVAL_SECS`
- `VM_CHECK_TIMEOUT_SECS`

## Examples

### Override HTTP Port

Using environment variable:
```bash
export HTTP_PORT=8080
nix develop -c cargo run
```

Using config.toml:
```toml
[network]
http_port = 8080
```

### Production Configuration

Create config.toml in project root:
```toml
[network]
http_port = 3020
http_bind_addr = "0.0.0.0"

[storage]
iroh_blobs_path = "/var/lib/blixard/iroh-blobs"
hiqlite_data_dir = "/var/lib/blixard/hiqlite"
vm_state_dir = "/var/lib/blixard/vm-state"
work_dir = "/tmp/blixard-work"

[vm]
default_memory_mb = 1024
default_vcpus = 2
max_concurrent_vms = 20
pre_warm_count = 5
```

### Development with Custom Settings

Use environment variables for quick overrides:
```bash
export FIRECRACKER_DEFAULT_MEMORY_MB=2048
export VM_PRE_WARM_COUNT=5
export HEALTH_CHECK_INTERVAL_SECS=10
nix develop -c cargo run
```

## API Usage

### Loading Configuration in Code

```rust
use mvm_ci::config::AppConfig;

// Load with layered approach (recommended)
let config = AppConfig::load_with_layers()?;

// Access configuration values
println!("HTTP Port: {}", config.network.http_port);
println!("VM Memory: {}MB", config.vm.default_memory_mb);

// Get duration values
let startup_timeout = config.timeouts.vm_startup_timeout();
let heartbeat_interval = config.timing.worker_heartbeat_interval();
```

### Configuration Validation

Configuration is validated on load. Invalid values will result in clear error messages:

```rust
match AppConfig::load_with_layers() {
    Ok(config) => {
        // Configuration is valid
        println!("Configuration loaded successfully");
    }
    Err(e) => {
        eprintln!("Configuration error: {}", e);
        std::process::exit(1);
    }
}
```

## Migration from Environment Variables

If you're currently using environment variables, no changes are required. The system is fully backward compatible:

1. All existing environment variables continue to work
2. Environment variables always override TOML configuration
3. You can gradually migrate to TOML files at your own pace

To migrate:
1. Copy config/default.toml to config.toml
2. Move environment variable values into config.toml
3. Keep environment variables for deployment-specific overrides

## Troubleshooting

### Configuration Not Loading

Check the logs for configuration loading messages:
```
INFO Loading configuration from: ./config.toml
```

or:
```
INFO No configuration file found, using hardcoded defaults
```

### Values Not Being Applied

Remember the precedence order:
1. Check if an environment variable is set (use `env | grep <KEY>`)
2. Verify the TOML syntax is correct
3. Ensure the configuration file is in a searched location

### Configuration File Not Found

Set CONFIG_FILE explicitly:
```bash
export CONFIG_FILE=/path/to/your/config.toml
```

Or place config.toml in the project root directory.

## Best Practices

1. **Use TOML for base configuration**: Put your standard configuration in config.toml
2. **Use environment variables for overrides**: Use env vars for deployment-specific values
3. **Document custom values**: Add comments to your config.toml explaining custom settings
4. **Version control example configs**: Check in example-*.toml files, but not config.toml
5. **Validate on startup**: Always check configuration is loaded correctly at startup

## File Locations

- `/home/brittonr/git/blixard/config/default.toml` - Default configuration with documentation
- `/home/brittonr/git/blixard/config/example-production.toml` - Production example
- `/home/brittonr/git/blixard/config.toml` - Your custom configuration (create this)
- `/home/brittonr/git/blixard/src/config.rs` - Configuration module implementation

## Support

For questions or issues:
1. Check this documentation
2. Review config/default.toml for all available options
3. Check the application logs for configuration loading messages
4. Review src/config.rs for implementation details
