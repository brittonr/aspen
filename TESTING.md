# Blixard Application Testing Guide

This document describes the integration test suite for the Blixard MVP application and how to validate its core functionality.

## Overview

The test suite validates:
1. Application startup with minimal configuration
2. HTTP server listening on the configured port
3. Health endpoint availability and response
4. Authentication middleware functionality
5. Generic job submission to the queue
6. Queue management endpoints (list, stats)
7. Payload validation and error handling

## Prerequisites

- Nix development environment
- Cargo and Rust toolchain (via `nix develop`)
- Port 3020 available (or customize via `HTTP_PORT` environment variable)

## Running the Tests

### Quick Start

```sh
./test_app.sh
```

### Custom Port

```sh
HTTP_PORT=8080 ./test_app.sh
```

### Environment Variables

The test script sets the following environment variables automatically:

- `SKIP_FLAWLESS=1` - Disables Flawless WASM backend
- `SKIP_VM_MANAGER=1` - Disables VM manager subsystem
- `BLIXARD_API_KEY` - Test API key (32+ characters)
- `HIQLITE_SECRET_RAFT` - Raft consensus secret
- `HIQLITE_SECRET_API` - Hiqlite API authentication secret
- `HIQLITE_ENC_KEY` - Database encryption key (base64)
- `HTTP_PORT` - HTTP server port (default: 3020)

## Test Cases

### Test 1: Health Check
- **Endpoint**: `GET /health/hiqlite`
- **Auth Required**: No
- **Expected**: HTTP 200 with health status JSON
- **Purpose**: Verify basic HTTP server functionality and database health

### Test 2: Authentication
- **Endpoint**: `POST /api/queue/publish` (without API key)
- **Auth Required**: Yes
- **Expected**: HTTP 401 Unauthorized
- **Purpose**: Verify API key authentication middleware

### Test 3: Generic JSON Payload
- **Endpoint**: `POST /api/queue/publish`
- **Payload**: `{"task": "process_data", "input": {"value": 42}}`
- **Expected**: HTTP 200 with job ID
- **Purpose**: Test simple structured data submission

### Test 4: Complex Nested Payload
- **Endpoint**: `POST /api/queue/publish`
- **Payload**: ETL workflow with nested steps
- **Expected**: HTTP 200 with job ID
- **Purpose**: Verify support for complex nested JSON structures

### Test 5: Simple String Payload
- **Endpoint**: `POST /api/queue/publish`
- **Payload**: `{"command": "echo Hello World"}`
- **Expected**: HTTP 200 with job ID
- **Purpose**: Test command-style job submission

### Test 6: Array-based Payload
- **Endpoint**: `POST /api/queue/publish`
- **Payload**: `{"items": [1,2,3,4,5], "operation": "sum"}`
- **Expected**: HTTP 200 with job ID
- **Purpose**: Verify array processing capabilities

### Test 7: URL-based Payload (Legacy)
- **Endpoint**: `POST /api/queue/publish`
- **Payload**: `{"url": "https://example.com/data", "method": "GET"}`
- **Expected**: HTTP 200 with job ID
- **Purpose**: Ensure backward compatibility with URL-based jobs

### Test 8: Queue List
- **Endpoint**: `GET /api/queue/list`
- **Auth Required**: Yes
- **Expected**: HTTP 200 with array of jobs
- **Purpose**: Verify queue inspection capability

### Test 9: Queue Statistics
- **Endpoint**: `GET /api/queue/stats`
- **Auth Required**: Yes
- **Expected**: HTTP 200 with stats object (pending, claimed, etc.)
- **Purpose**: Validate queue metrics reporting

### Test 10: Null Payload Rejection
- **Endpoint**: `POST /api/queue/publish`
- **Payload**: `null`
- **Expected**: HTTP 400 or 500 with error message
- **Purpose**: Verify payload validation

## API Reference

### Authentication

All API endpoints under `/api/*` require authentication via the `X-API-Key` header:

```sh
curl -H "X-API-Key: your_secret_key_here" \
     http://localhost:3020/api/queue/stats
```

### Health Endpoint

No authentication required:

```sh
curl http://localhost:3020/health/hiqlite
```

Example response:
```json
{
  "is_healthy": true,
  "node_count": 1,
  "has_leader": true
}
```

### Job Submission

```sh
curl -X POST \
     -H "X-API-Key: your_secret_key_here" \
     -H "Content-Type: application/json" \
     -d '{"task": "process", "data": {"value": 123}}' \
     http://localhost:3020/api/queue/publish
```

Example response:
```
Job job-1 published
```

### Queue List

```sh
curl -H "X-API-Key: your_secret_key_here" \
     http://localhost:3020/api/queue/list
```

Example response:
```json
[
  {
    "id": "job-1",
    "status": "Pending",
    "payload": {"task": "process", "data": {"value": 123}},
    "created_at": 1700000000,
    "updated_at": 1700000000,
    "claimed_by": null,
    "assigned_worker_id": null,
    "completed_by": null,
    "started_at": null,
    "error_message": null,
    "retry_count": 0,
    "compatible_worker_types": []
  }
]
```

### Queue Statistics

```sh
curl -H "X-API-Key: your_secret_key_here" \
     http://localhost:3020/api/queue/stats
```

Example response:
```json
{
  "total": 5,
  "pending": 3,
  "claimed": 1,
  "in_progress": 1,
  "completed": 0,
  "failed": 0
}
```

## What Works

### Core Functionality
- ✓ Application starts successfully with minimal environment variables
- ✓ HTTP server listens on configured port (default: 3020)
- ✓ Health endpoint responds correctly
- ✓ API key authentication middleware enforces security
- ✓ Generic JSON payloads are accepted and queued
- ✓ Complex nested structures are supported
- ✓ Queue inspection endpoints provide visibility
- ✓ Payload validation rejects invalid input

### Architecture
- ✓ Clean domain layer separation (no infrastructure leakage)
- ✓ Repository pattern for data access abstraction
- ✓ Service layer handles business logic
- ✓ Middleware properly applied to route groups
- ✓ Modular router structure with clear boundaries

### Skipped Components
The following components are intentionally disabled for MVP testing:
- Flawless WASM backend (`SKIP_FLAWLESS=1`)
- VM Manager subsystem (`SKIP_VM_MANAGER=1`)

These will be tested separately once the core queue functionality is stable.

## What Still Needs Improvement

### Job Execution
- **Status**: Not tested
- **Issue**: No worker nodes available to claim and execute jobs
- **Next Steps**:
  - Test worker registration
  - Test job claiming by workers
  - Test job execution and status updates
  - Test job completion workflow

### Job Status Updates
- **Status**: Not tested
- **Issue**: Endpoint exists but requires active workers
- **Endpoint**: `POST /api/queue/status/{job_id}`
- **Next Steps**: Test status transitions (Pending → Claimed → InProgress → Completed/Failed)

### Error Handling
- **Status**: Partially tested
- **Coverage**: Null payload validation works
- **Missing**:
  - Invalid JSON structure handling
  - Database connection failures
  - Concurrent access edge cases
  - Retry logic validation

### Worker Management
- **Status**: Not tested
- **Endpoints**:
  - `POST /api/workers/register`
  - `POST /api/workers/{worker_id}/heartbeat`
  - `GET /api/workers`
  - `POST /api/workers/{worker_id}/drain`
- **Next Steps**: Create worker integration tests

### Performance & Load Testing
- **Status**: Not tested
- **Missing**:
  - Concurrent job submission stress test
  - High-throughput queue processing
  - Large payload handling
  - Database query performance under load
  - Memory leak detection

### Dashboard UI
- **Status**: Not tested
- **Endpoints**: `/dashboard/*`
- **Next Steps**: Manual browser testing of HTMX UI

### Iroh P2P Functionality
- **Status**: Not tested
- **Endpoints**: `/api/iroh/*`
- **Next Steps**: Test blob storage and gossip protocol

### OpenTofu State Backend
- **Status**: Not tested (feature-gated)
- **Endpoints**: `/api/tofu/*`
- **Next Steps**: Test with actual Terraform/OpenTofu client

## Troubleshooting

### Port Already in Use

```sh
# Find process using the port
lsof -i :3020
# Or
ss -tuln | grep 3020

# Kill the process or use a different port
HTTP_PORT=8080 ./test_app.sh
```

### Application Won't Start

Check the log file:
```sh
tail -f test_app.log
```

Common issues:
- Missing Hiqlite secrets (check environment variables)
- Database initialization failure
- Port permission issues (use port > 1024)

### Tests Failing

1. Check application is running: `ps aux | grep mvm-ci`
2. Test health endpoint manually: `curl http://localhost:3020/health/hiqlite`
3. Verify API key is set: `echo $BLIXARD_API_KEY`
4. Check application logs: `cat test_app.log`

### Database Errors

If you see Hiqlite errors:
1. Ensure secrets are at least 32 characters
2. Check file permissions on `./data/hiqlite` directory
3. Try cleaning the data directory: `rm -rf ./data/hiqlite`

## Future Test Coverage

### Integration Tests Needed
- [ ] Worker lifecycle (register, heartbeat, drain, offline)
- [ ] Job claiming by workers with worker type filtering
- [ ] Job execution state machine transitions
- [ ] Retry logic for failed jobs
- [ ] Job timeout handling
- [ ] Concurrent job submission and claiming
- [ ] Dashboard UI rendering and interaction
- [ ] Iroh blob storage and retrieval
- [ ] Iroh gossip pub/sub
- [ ] OpenTofu state backend protocol
- [ ] OpenTofu locking mechanism
- [ ] Error recovery scenarios

### Performance Tests Needed
- [ ] 1000 concurrent job submissions
- [ ] Queue processing throughput
- [ ] Large payload handling (10MB+)
- [ ] Database query performance profiling
- [ ] Memory usage under sustained load
- [ ] Connection pool exhaustion handling

### Security Tests Needed
- [ ] API key brute force protection
- [ ] SQL injection attempts (if applicable)
- [ ] Large payload DoS protection
- [ ] Rate limiting enforcement
- [ ] CORS policy validation

## Manual Testing Checklist

After automated tests pass, manually verify:

1. **Dashboard UI** (browser)
   - [ ] Navigate to http://localhost:3020/dashboard
   - [ ] View cluster health status
   - [ ] View queue statistics
   - [ ] View recent jobs list
   - [ ] Submit job via web form
   - [ ] HTMX auto-refresh works

2. **Worker Registration** (curl or worker binary)
   - [ ] Register Firecracker worker
   - [ ] Register WASM worker
   - [ ] Send heartbeat
   - [ ] Worker appears in dashboard

3. **Job Execution** (end-to-end)
   - [ ] Submit job via API
   - [ ] Worker claims job
   - [ ] Job transitions to InProgress
   - [ ] Job completes or fails
   - [ ] Results visible in dashboard

4. **Error Scenarios**
   - [ ] Kill worker mid-job (job should retry)
   - [ ] Submit invalid payload
   - [ ] Exceed rate limits
   - [ ] Database connection loss recovery

## Continuous Improvement

As bugs are discovered and fixed:
1. Add regression test to `test_app.sh`
2. Document the issue and resolution here
3. Update test coverage metrics
4. Re-run full test suite

## Metrics

Current test coverage:
- **API Endpoints**: 5/15 tested (33%)
- **Job Lifecycle**: 1/5 stages tested (20%)
- **Worker Lifecycle**: 0/4 stages tested (0%)
- **Error Scenarios**: 2/10 tested (20%)

Target for MVP release:
- **API Endpoints**: 100% core endpoints
- **Job Lifecycle**: 100% happy path + critical errors
- **Worker Lifecycle**: 100% happy path
- **Error Scenarios**: 80% coverage
