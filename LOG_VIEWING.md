# Cluster Log Viewing

This directory includes several scripts for monitoring the distributed cluster:

## Log Aggregation Scripts

### 1. **view-cluster-logs.sh** - All Container Logs
View aggregated logs from all Docker containers with color-coded output.

```bash
./view-cluster-logs.sh          # All nodes
./view-cluster-logs.sh "1 2"    # Only nodes 1 and 2
./view-cluster-logs.sh "3"      # Only node 3
```

Output includes:
- Container startup logs
- mvm-ci application logs
- Hiqlite Raft consensus logs
- Iroh P2P networking logs

### 2. **view-flawless-logs.sh** - Workflow Execution Logs
View only the Flawless WASM workflow execution logs from all nodes.

```bash
./view-flawless-logs.sh
```

Shows:
- Workflow startup/shutdown
- WASM module loading
- Workflow execution output
- Runtime errors

### 3. **view-workflow-activity.sh** - Active Job Processing
Real-time monitor showing only workflow job processing activity.

```bash
./view-workflow-activity.sh
```

Shows:
- Job start/finish events
- Processing iterations
- Which node is working on which job

## Color Coding

Each node has a distinct color:
- **Node 1**: Green
- **Node 2**: Blue
- **Node 3**: Magenta

## Examples

### Monitor cluster while submitting jobs:

Terminal 1:
```bash
./view-workflow-activity.sh
```

Terminal 2:
```bash
curl -X POST http://localhost:3020/new-job \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "url=https://example.com"
```

### Debug a specific node:

```bash
./view-cluster-logs.sh "2"  # Only node 2
```

### Watch flawless server activity:

```bash
./view-flawless-logs.sh
```

## Tips

- All scripts support Ctrl+C to stop
- Logs are streamed in real-time (using `docker logs -f`)
- Use `grep` to filter further: `./view-cluster-logs.sh | grep ERROR`
- Combine with `tee` to save while viewing: `./view-cluster-logs.sh | tee cluster.log`

## Direct Docker Commands

For advanced usage:

```bash
# Individual container logs
docker logs -f mvm-ci-node1
docker logs -f mvm-ci-node2
docker logs -f mvm-ci-node3

# Flawless logs inside container
docker exec mvm-ci-node1 cat /var/log/flawless.log

# Follow flawless logs
docker exec mvm-ci-node1 tail -f /var/log/flawless.log
```
