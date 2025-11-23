# MVM-CI Quick Start Guide

## Starting the Cluster

```bash
# Build the Docker image
nix build .#dockerImage

# Run the test (starts cluster and runs test jobs)
./test-option-a.sh
```

## Monitoring

### Interactive Monitor (Recommended)
```bash
./monitor-cluster.sh
```

Features:
- **Auto-refresh** every 2 seconds
- **Color-coded** status (green = good, red = error, yellow = pending)
- Shows:
  - Cluster health
  - Queue statistics
  - Recent jobs
  - Worker activity

**Commands:**
- `j` - Submit a test job
- `l` - View container logs
- `r` - Refresh now
- `q` - Quit

### Quick Snapshot
```bash
./monitor-cluster.sh --once
```

### Manual Commands

**Check queue:**
```bash
curl -s http://localhost:3020/queue/list | jq
```

**Check stats:**
```bash
curl -s http://localhost:3020/queue/stats | jq
```

**Submit a job:**
```bash
curl -X POST http://localhost:3020/new-job \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "url=https://example.com"
```

**View logs:**
```bash
# Control plane
docker logs -f mvm-ci-node1

# Workers
docker logs -f mvm-ci-worker1
docker logs -f mvm-ci-worker2
```

## Architecture

**Control Plane:**
- 3 nodes running Raft consensus (hiqlite)
- Manages job queue state
- Exposes HTTP API on `localhost:3020`
- Exposes iroh+h3 P2P endpoint for workers

**Workers:**
- 2 workers with isolated Flawless instances
- Connect via iroh+h3 P2P networking
- Poll for jobs every 2 seconds
- Execute WASM workflows in isolation

## Cleanup

```bash
docker compose down -v
```

## Troubleshooting

**Workers not claiming jobs?**
- Check worker logs: `docker logs mvm-ci-worker1`
- Verify control plane ticket is set: `docker logs mvm-ci-node1 | grep "iroh+h3://endpoint"`
- Check health: `curl http://localhost:3020/hiqlite/health | jq`

**Control plane unhealthy?**
- Wait 15 seconds for Raft cluster to form
- Check logs: `docker logs mvm-ci-node1`
- Verify all 3 nodes are running: `docker ps | grep node`

**Docker image not loading?**
- Make sure to run: `./result | docker load` (not `docker load < ./result`)
- The result is an executable script from Nix's `streamLayeredImage`

## Development

**Rebuild after code changes:**
```bash
# Force rebuild
nix build .#dockerImage --rebuild

# Or with garbage collection first
nix-collect-garbage && nix build .#dockerImage --rebuild
```

**Run tests:**
```bash
cargo nextest run
```

**Format code:**
```bash
cargo fmt
nix fmt
```
