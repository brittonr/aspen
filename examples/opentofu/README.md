# OpenTofu/Terraform State Backend Integration

This example demonstrates how to use Blixard as a remote state backend for OpenTofu/Terraform.

## Features

- **Distributed State Management**: State is stored in Hiqlite with Raft consensus
- **State Locking**: Prevents concurrent modifications with distributed locks
- **State History**: Automatic versioning and rollback capabilities
- **Plan Execution**: Execute OpenTofu plans through Blixard's execution backend

## Setup

### 1. Start Blixard Control Plane

Ensure Blixard is running with Hiqlite enabled:

```bash
# Start Blixard with default configuration
cargo run --bin mvm-ci

# Or with custom Hiqlite configuration
HQL_NODE_ID=1 HQL_SECRET_RAFT=secret123 HQL_SECRET_API=api123 cargo run --bin mvm-ci
```

### 2. Initialize OpenTofu

```bash
# Navigate to this example directory
cd examples/opentofu

# Initialize OpenTofu with the HTTP backend
tofu init

# You should see:
# Initializing the backend...
# Successfully configured the backend "http"!
```

### 3. Plan and Apply

```bash
# Create an execution plan
tofu plan

# Apply the changes
tofu apply
```

## State Management Operations

### View Current State
```bash
# Using OpenTofu
tofu show

# Using Blixard API
curl -X GET http://localhost:8080/api/tofu/state/default
```

### List Workspaces
```bash
curl -X GET http://localhost:8080/api/tofu/workspaces
```

### State History
```bash
# View state history
curl -X GET http://localhost:8080/api/tofu/history/default

# Rollback to a previous version
curl -X POST http://localhost:8080/api/tofu/rollback/default/2
```

### Lock Management
```bash
# Force unlock (if needed)
curl -X DELETE http://localhost:8080/api/tofu/lock/default
```

## Plan Execution via API

Instead of running OpenTofu locally, you can submit plans for execution through Blixard:

### Create and Execute Plan
```bash
curl -X POST http://localhost:8080/api/tofu/plan \
  -H "Content-Type: application/json" \
  -d '{
    "workspace": "default",
    "config_dir": "/path/to/terraform/config",
    "auto_approve": false
  }'
```

### Apply Stored Plan
```bash
curl -X POST http://localhost:8080/api/tofu/apply/{plan_id} \
  -H "Content-Type: application/json" \
  -d '{
    "approver": "admin"
  }'
```

### List Plans
```bash
curl -X GET http://localhost:8080/api/tofu/plans/default
```

## Configuration

### Environment Variables for Authentication

If API key authentication is enabled in Blixard:

```bash
export TF_HTTP_USERNAME=api
export TF_HTTP_PASSWORD=your-api-key-here
```

### Backend Configuration Options

```hcl
backend "http" {
  address         = "http://localhost:8080/api/tofu/state/workspace-name"
  lock_address    = "http://localhost:8080/api/tofu/lock/workspace-name"
  unlock_address  = "http://localhost:8080/api/tofu/unlock/workspace-name"

  # Optional settings
  update_method   = "POST"        # Method for state updates (default: POST)
  lock_method     = "POST"         # Method for locking (default: LOCK)
  unlock_method   = "POST"         # Method for unlocking (default: UNLOCK)

  # Retry configuration
  retry_wait_min  = 1              # Min seconds between retries
  retry_wait_max  = 3              # Max seconds between retries
  retry_max       = 5              # Max number of retries

  # Authentication
  username        = "api"          # Or use TF_HTTP_USERNAME
  password        = "key"          # Or use TF_HTTP_PASSWORD
}
```

## Advantages of Using Blixard as State Backend

1. **High Availability**: Raft consensus ensures state consistency across multiple nodes
2. **No External Dependencies**: State is managed within your existing Blixard infrastructure
3. **Integrated Execution**: Plans can be executed through Blixard's execution backend
4. **Security**: API key authentication and workspace isolation
5. **Observability**: State history, audit trails, and metrics
6. **Scalability**: Distributed architecture supports large-scale deployments

## Troubleshooting

### State Lock Issues
If you encounter state lock errors:
1. Check if another operation is running: `curl -X GET http://localhost:8080/api/tofu/lock/default`
2. Force unlock if needed: `curl -X DELETE http://localhost:8080/api/tofu/lock/default`

### Connection Issues
Ensure Blixard is running and accessible:
```bash
curl -X GET http://localhost:8080/health/hiqlite
```

### State Version Conflicts
If you get version mismatch errors, check the current version:
```bash
curl -X GET http://localhost:8080/api/tofu/state/default | jq '.serial'
```