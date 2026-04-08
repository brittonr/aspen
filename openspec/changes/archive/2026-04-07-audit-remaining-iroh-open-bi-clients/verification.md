# Verification Evidence

## OpenSpec Status (before archive)

```json
{
  "changeName": "audit-remaining-iroh-open-bi-clients",
  "schemaName": "spec-driven",
  "isComplete": true,
  "artifacts": [
    { "id": "proposal", "outputPath": "proposal.md", "status": "done" },
    { "id": "design", "outputPath": "design.md", "status": "done" },
    { "id": "specs", "outputPath": "specs/**/*.md", "status": "done" },
    { "id": "tasks", "outputPath": "tasks.md", "status": "done" }
  ]
}
```

## Tasks 1.1–3.3: Production Implementation (commit be87938aa)

Commit `be87938aa927c9c7d27eca328759e839fb506ae3` — "Bound all remaining direct
open_bi() request/response clients" — 612 insertions, 143 deletions across 12 files
(11 source files + tasks.md).

Every source file below had bare `open_bi()` → `write_all()` → `finish()` →
`read_to_end()` replaced with `timeout(remaining_timeout_*(deadline)?, ...)` per stage.

### 2.1 src/bin/git-remote-aspen/main.rs (+50 lines)

```diff
-        let (mut send, mut recv) = connection.open_bi().await.map_err(io::Error::other)?;
-        send.write_all(&request_bytes).await.map_err(io::Error::other)?;
-        send.finish().map_err(io::Error::other)?;
+        let deadline = Instant::now() + RPC_TIMEOUT;
+        let (mut send, mut recv) = timeout(remaining_timeout_io(deadline)?, connection.open_bi())
+            .await.map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "stream open timeout"))?
+            .map_err(io::Error::other)?;
+        timeout(remaining_timeout_io(deadline)?, send.write_all(&request_bytes))
+            .await.map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "request write timeout"))?
+            .map_err(io::Error::other)?;
+        timeout(remaining_timeout_io(deadline)?, async { send.finish().map_err(io::Error::other) })
+            .await.map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "stream finish timeout"))??;
+        let response_bytes = timeout(remaining_timeout_io(deadline)?, recv.read_to_end(MAX_GIT_FETCH_RESPONSE_SIZE))
+            .await.map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "response timeout"))?
+            .map_err(io::Error::other)?;
```

### 2.2 crates/aspen-tui/src/iroh_client/rpc.rs (+152 lines)

```diff
-        let connection = self.endpoint.connect(target_addr, CLIENT_ALPN).await...
-        let (mut send, mut recv) = connection.open_bi().await...
-        send.write_all(&request_bytes).await...
-        send.finish()...
+        let connection = timeout(RPC_TIMEOUT, self.endpoint.connect(target_addr, CLIENT_ALPN))...
+        let deadline = Instant::now() + RPC_TIMEOUT;
+        let (mut send, mut recv) = timeout(remaining_timeout(deadline)?, connection.open_bi())...
+        timeout(remaining_timeout(deadline)?, send.write_all(&request_bytes))...
+        timeout(remaining_timeout(deadline)?, async { send.finish()... })...
+        let response_bytes = timeout(remaining_timeout(deadline)?, recv.read_to_end(...))...
```

### 2.2 crates/aspen-tui/src/iroh_client/multi_node.rs (+50 lines)

```diff
-        let connection =
-            self.endpoint.connect(target.clone(), CLIENT_ALPN).await...
-        let (mut send, mut recv) = connection.open_bi().await...
-        send.write_all(&request_bytes).await...
-        send.finish()...
+        let connection = tokio::time::timeout(RPC_TIMEOUT, self.endpoint.connect(target.clone(), CLIENT_ALPN))...
+        let deadline = Instant::now() + RPC_TIMEOUT;
+        let (mut send, mut recv) = tokio::time::timeout(remaining_timeout(deadline)?, connection.open_bi())...
+        tokio::time::timeout(remaining_timeout(deadline)?, send.write_all(&request_bytes))...
+        tokio::time::timeout(remaining_timeout(deadline)?, async { send.finish()... })...
+        tokio::time::timeout(remaining_timeout(deadline)?, recv.read_to_end(...))...
```

### 2.3 crates/aspen-fuse/src/client.rs (+40 lines)

```diff
-        let (mut send, mut recv) = match connection.open_bi().await {
+        let deadline = Instant::now() + READ_TIMEOUT;
+        let (mut send, mut recv) = match tokio::time::timeout(remaining_timeout(deadline)?, connection.open_bi())
+            .await.context("stream open timeout")? {
-        send.write_all(&request_bytes).await...
-        send.finish()...
+        tokio::time::timeout(remaining_timeout(deadline)?, send.write_all(&request_bytes))...
+        tokio::time::timeout(remaining_timeout(deadline)?, async { send.finish()... })...
+        tokio::time::timeout(remaining_timeout(deadline)?, recv.read_to_end(...))...
```

### 2.3 crates/aspen-hooks/src/client.rs (+45 lines)

```diff
-        let (mut send, mut recv) = connection.open_bi().await...
-        send.write_all(&request_bytes).await...
-        send.finish()...
+        let deadline = std::time::Instant::now() + self.timeout;
+        let (mut send, mut recv) = timeout(remaining_timeout_hook(deadline)?, connection.open_bi())...
+        timeout(remaining_timeout_hook(deadline)?, send.write_all(&request_bytes))...
+        timeout(remaining_timeout_hook(deadline)?, async { send.finish()... })...
+        timeout(remaining_timeout_hook(deadline)?, recv.read_to_end(...))...
```

### 2.3 crates/aspen-cli/src/bin/aspen-cli/commands/hooks.rs (+43 lines)

```diff
-    let (mut send, mut recv) = connection.open_bi().await...
-    send.write_all(&request_bytes).await...
-    send.finish()...
+    let deadline = std::time::Instant::now() + rpc_timeout;
+    let (mut send, mut recv) = timeout(remaining_timeout_hooks(deadline)?, connection.open_bi())...
+    timeout(remaining_timeout_hooks(deadline)?, send.write_all(&request_bytes))...
+    timeout(remaining_timeout_hooks(deadline)?, async { send.finish()... })...
+    timeout(remaining_timeout_hooks(deadline)?, recv.read_to_end(...))...
```

### 3.1 crates/aspen-snix/src/rpc_blob_service.rs (+87 lines)

```diff
-        let (mut send, mut recv) = connection.open_bi().await...
-        send.write_all(&request_bytes).await...
+        let deadline = std::time::Instant::now() + RPC_TIMEOUT;
+        let (mut send, mut recv) = tokio::time::timeout(remaining_timeout_io(deadline)?, connection.open_bi())...
+        tokio::time::timeout(remaining_timeout_io(deadline)?, send.write_all(&request_bytes))...
+        tokio::time::timeout(remaining_timeout_io(deadline)?, async { send.finish()... })...
+        tokio::time::timeout(remaining_timeout_io(deadline)?, recv.read_to_end(MAX_RESPONSE_SIZE))...
```

### 3.1 crates/aspen-snix/src/rpc_directory_service.rs (+68 lines)

```diff
-        let (send, recv) = connection.open_bi().await.map_err(|e| {
-            Error::from(format!("failed to open RPC stream: {e}"))
-        })?;
-        send.write_all(&request_bytes).await...
+        let deadline = std::time::Instant::now() + RPC_TIMEOUT;
+        let (send, recv) = timeout(remaining_timeout_dir(deadline)?, connection.open_bi())
+            .await.map_err(|_| Error::from("stream open timeout"))?
+            .map_err(|e| Error::from(format!("failed to open RPC stream: {e}")))?;
+        timeout(remaining_timeout_dir(deadline)?, send.write_all(&request_bytes))...
+        timeout(remaining_timeout_dir(deadline)?, async { send.finish()... })...
+        timeout(remaining_timeout_dir(deadline)?, recv.read_to_end(...))...
```

### 3.1 crates/aspen-snix/src/rpc_pathinfo_service.rs (+68 lines)

```diff
-        let (mut send, mut recv) = connection.open_bi().await.map_err(|e| {
-            Box::new(std::io::Error::other(format!("failed to open RPC stream: {}", e))) as Error
-        })?;
-        send.write_all(&request_bytes).await...
+        let deadline = std::time::Instant::now() + RPC_TIMEOUT;
+        let (mut send, mut recv) = timeout(remaining_timeout_pathinfo(deadline)?, connection.open_bi())
+            .await.map_err(|_| Box::new(std::io::Error::new(std::io::ErrorKind::TimedOut, "stream open timeout")) as Error)?
+            .map_err(|e| Box::new(std::io::Error::other(format!("failed to open RPC stream: {}", e))) as Error)?;
+        timeout(remaining_timeout_pathinfo(deadline)?, send.write_all(&request_bytes))...
+        timeout(remaining_timeout_pathinfo(deadline)?, async { send.finish()... })...
+        timeout(remaining_timeout_pathinfo(deadline)?, recv.read_to_end(...))...
```

### 3.2 crates/aspen-blob/src/replication/adapters.rs (+117 lines)

```diff
-        let (mut send, mut recv) = connection.open_bi().await...
-        send.write_all(&request_bytes).await...
-        send.finish()...
+        let deadline = std::time::Instant::now() + RPC_TIMEOUT;
+        let (mut send, mut recv) = tokio::time::timeout(remaining_timeout_blob(deadline)?, connection.open_bi())...
+        tokio::time::timeout(remaining_timeout_blob(deadline)?, send.write_all(&request_bytes))...
+        tokio::time::timeout(remaining_timeout_blob(deadline)?, async { send.finish()... })...
+        tokio::time::timeout(remaining_timeout_blob(deadline)?, recv.read_to_end(...))...
```

### 3.3 crates/aspen-castore/src/client.rs (+17 lines)

```diff
+        const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
-            let conn = inner.endpoint.connect(inner.addr.clone(), &inner.alpn).await...
+            let conn = tokio::time::timeout(CONNECT_TIMEOUT, inner.endpoint.connect(...))
+                .await.map_err(|_| -> irpc::RequestError {
+                    io::Error::new(io::ErrorKind::TimedOut, "iroh connect timeout").into()
+                })?...
```

Decision: castore owns a 5s connect timeout; the IRPC layer owns the overall exchange budget.

## Task 4.1: Regression Test Output

### git-remote-aspen (user-facing bucket)

```
Nextest run ID bc769596 with nextest profile: default
    Starting 4 tests across 26 binaries (989 tests skipped)
        PASS [   0.003s] aspen::bin/git-remote-aspen tests::expired_deadline_blocks_all_stages
        PASS [   2.432s] aspen::bin/git-remote-aspen tests::full_exchange_times_out_when_peer_never_replies
        PASS [   0.004s] aspen::bin/git-remote-aspen tests::remaining_timeout_io_returns_duration_when_deadline_is_ahead
        PASS [   0.003s] aspen::bin/git-remote-aspen tests::remaining_timeout_io_returns_timed_out_when_deadline_is_past
     Summary [   2.445s] 4 tests run: 4 passed, 989 skipped
```

### aspen-snix rpc_blob_service (bridge/service bucket)

```
    Starting 5 tests across 8 binaries (131 tests skipped)
        PASS [   0.224s] aspen-snix rpc_blob_service::tests::connect_timeout_on_unreachable_peer
        PASS [   0.003s] aspen-snix rpc_blob_service::tests::expired_deadline_blocks_all_stages
        PASS [   2.387s] aspen-snix rpc_blob_service::tests::full_exchange_times_out_when_peer_never_replies
        PASS [   0.003s] aspen-snix rpc_blob_service::tests::remaining_timeout_io_returns_duration_when_deadline_is_ahead
        PASS [   0.003s] aspen-snix rpc_blob_service::tests::remaining_timeout_io_returns_timed_out_when_deadline_is_past
     Summary [   2.621s] 5 tests run: 5 passed, 131 skipped
```

## Task 4.2: Targeted Crate Test Output

```
=== aspen-tui (rpc.rs + multi_node.rs) ===
        PASS [   0.003s] aspen-tui::bin/aspen-tui commands::tests::test_c_key_context_sensitive
        PASS [   0.003s] aspen-tui::bin/aspen-tui commands::tests::test_editing_mode
        PASS [   0.002s] aspen-tui::bin/aspen-tui commands::tests::test_esc_context_sensitive
        PASS [   0.002s] aspen-tui::bin/aspen-tui commands::tests::test_quit_command
        PASS [   0.003s] aspen-tui::bin/aspen-tui commands::tests::test_sql_editing_mode
        PASS [   0.002s] aspen-tui::bin/aspen-tui commands::tests::test_view_navigation
        PASS [   0.002s] aspen-tui::bin/aspen-tui iroh_client::rpc::tests::expired_deadline_blocks_all_stages
        PASS [   0.002s] aspen-tui::bin/aspen-tui iroh_client::rpc::tests::remaining_timeout_returns_duration_when_deadline_is_ahead
        PASS [   0.003s] aspen-tui::bin/aspen-tui iroh_client::rpc::tests::remaining_timeout_returns_error_when_deadline_is_past
        PASS [   0.222s] aspen-tui::bin/aspen-tui iroh_client::rpc::tests::send_rpc_connect_timeout_on_unreachable_peer
        PASS [   0.015s] aspen-tui::bin/aspen-tui iroh_client::rpc::tests::stage_finish_bounded_by_deadline
        PASS [   0.015s] aspen-tui::bin/aspen-tui iroh_client::rpc::tests::stage_open_bi_bounded_by_deadline
        PASS [   0.014s] aspen-tui::bin/aspen-tui iroh_client::rpc::tests::stage_read_bounded_by_deadline
        PASS [   0.014s] aspen-tui::bin/aspen-tui iroh_client::rpc::tests::stage_write_bounded_by_deadline
     Summary [   0.303s] 14 tests run: 14 passed, 0 skipped

=== aspen-fuse (client.rs) ===
     Summary [  16.055s] 202 tests run: 202 passed, 0 skipped

=== aspen-hooks (client.rs) ===
        PASS [   0.003s] aspen-hooks client::tests::test_invalid_prefix
        PASS [   0.002s] aspen-hooks client::tests::test_invalid_url
     Summary [   0.006s] 2 tests run: 2 passed, 136 skipped

=== aspen-cli (commands/hooks.rs + client.rs) ===
     Summary [   6.557s] 289 tests run: 289 passed, 0 skipped

=== aspen-snix (blob + directory + pathinfo services) ===
     Summary [   5.828s] 136 tests run: 136 passed, 0 skipped

=== aspen-blob (replication/adapters.rs) ===
        PASS [   0.228s] aspen-blob replication::adapters::remaining_timeout_tests::connect_timeout_on_unreachable_peer
        PASS [   0.003s] aspen-blob replication::adapters::remaining_timeout_tests::expired_deadline_blocks_all_stages
        PASS [   0.003s] aspen-blob replication::adapters::remaining_timeout_tests::remaining_timeout_blob_returns_duration_when_deadline_is_ahead
        PASS [   0.003s] aspen-blob replication::adapters::remaining_timeout_tests::remaining_timeout_blob_returns_error_when_deadline_is_past
        PASS [   0.047s] aspen-blob replication::adapters::remaining_timeout_tests::stages_bounded_by_deadline
     Summary [   0.285s] 5 tests run: 5 passed, 63 skipped

=== aspen-castore (client.rs) ===
     Summary [   0.164s] 14 tests run: 14 passed, 16 skipped
```

Total across all 8 crates: 662 tests, 0 failures.

Note: the `aspen` root crate (containing `git-remote-aspen`) is covered in task 4.1
above (4 tests). The 7 crates listed here plus that 1 crate = 8 crates total.

## Task 4.3: Live Cluster Command

Started 1-node cluster (`aspen-node --node-id 1 --cookie live-test --data-dir /dev/shm/aspen-live-test/node1`),
initialized with `aspen-cli cluster init`, then ran:

```
$ aspen-cli --ticket aspennv2ioc5ce... --timeout 10 kv set timeout-test-key "hello-from-hardened-cli"
OK
EXIT: 0

$ aspen-cli --ticket aspennv2ioc5ce... --timeout 10 kv get timeout-test-key
hello-from-hardened-cli
EXIT: 0

$ aspen-cli --ticket aspennv2ioc5ce... --timeout 10 cluster health
Health Status
=============
Status:         healthy
Node ID:        1
Raft Node ID:   1
Iroh Node ID:   8cd0c9537d3c3b0af471c364a1f2716810cbd31e82527f3eba80a83f6260bdab
Uptime:         16s
EXIT: 0
```

Each command exercised the full hardened QUIC exchange: connect → open_bi → write → finish → read.

## Archived tasks.md Checklist

```markdown
## 1. Confirm the follow-up inventory

- [x] 1.1 Re-run the direct `open_bi()` audit and confirm the one-request/one-response clients still lacking full timeout coverage
- [x] 1.2 Separate long-lived/session protocols and test-only call sites from the request/response follow-up list
- [x] 1.3 Decide which bridge helpers need dedicated large-response read budgets

## 2. Harden the remaining user-facing clients

- [x] 2.1 Update `src/bin/git-remote-aspen/main.rs`
- [x] 2.2 Update `crates/aspen-tui/src/iroh_client/rpc.rs` and `multi_node.rs`
- [x] 2.3 Update `crates/aspen-fuse/src/client.rs`, `crates/aspen-hooks/src/client.rs`, and `crates/aspen-cli/src/bin/aspen-cli/commands/hooks.rs`

## 3. Harden the remaining bridge/service clients

- [x] 3.1 Update `crates/aspen-snix/src/rpc_blob_service.rs`, `rpc_directory_service.rs`, and `rpc_pathinfo_service.rs`
- [x] 3.2 Update `crates/aspen-blob/src/replication/adapters.rs`
- [x] 3.3 Decide whether `crates/aspen-castore/src/client.rs` should own stream-open timeouts or delegate them to the IRPC layer

## 4. Verify the follow-up rollout

- [x] 4.1 Add targeted timeout regression tests for at least one representative client from each bucket
- [x] 4.2 Run targeted crate tests for the touched clients
- [x] 4.3 Run one quick live command path through a hardened auxiliary client
```
