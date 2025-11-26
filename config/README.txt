Blixard Configuration Files
===========================

This directory contains configuration file examples for Blixard.

Configuration Files:
--------------------
- default.toml          : Default configuration with sensible defaults and extensive documentation
- example-production.toml : Example production configuration (copy to config.toml)

Configuration Loading:
---------------------
Blixard loads configuration with the following precedence (highest to lowest):

1. Environment variables (always override everything)
2. CONFIG_FILE environment variable (path to custom config file)
3. ./config.toml (project root)
4. ./config/default.toml (this directory)
5. Hardcoded defaults in the application

Usage:
------
1. For development: The application will automatically use ./config/default.toml

2. For production:
   - Copy example-production.toml to ../config.toml (project root)
   - Customize values as needed
   - Or set CONFIG_FILE=/path/to/your/config.toml

3. Override specific values with environment variables:
   export HTTP_PORT=8080
   export VM_DEFAULT_MEMORY_MB=2048

Environment Variables:
---------------------
All configuration values can be overridden with environment variables.
See default.toml for the complete list of available options.

Examples:
---------
# Network
HTTP_PORT=3020
HTTP_BIND_ADDR="0.0.0.0"
IROH_ALPN="iroh+h3"

# Storage
IROH_BLOBS_PATH="./data/iroh-blobs"
HQL_DATA_DIR="./data/hiqlite"
VM_STATE_DIR="./data/vm-state"
WORK_DIR="/tmp/tofu-work"

# Flawless
FLAWLESS_URL="http://localhost:27288"

# VM Configuration
FIRECRACKER_FLAKE_DIR="./microvms"
FIRECRACKER_STATE_DIR="./data/firecracker-vms"
FIRECRACKER_DEFAULT_MEMORY_MB=512
FIRECRACKER_DEFAULT_VCPUS=1
FIRECRACKER_MAX_CONCURRENT_VMS=10
VM_AUTO_SCALING=true
VM_PRE_WARM_COUNT=2

# Timeouts
VM_STARTUP_TIMEOUT_SECS=30
VM_SHUTDOWN_TIMEOUT_SECS=30
ADAPTER_DEFAULT_TIMEOUT_SECS=300

For a complete list of environment variables, see the default.toml file
which documents all available configuration options.
