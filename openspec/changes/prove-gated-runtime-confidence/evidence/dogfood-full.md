# Dogfood full evidence

Captured: 2026-05-13T03:56:25Z

Raw logs are kept under ignored `target/runtime-proof/dogfood-full.log` and `target/runtime-proof/dogfood-full-datapool.log`. Committed evidence redacts tickets and omits raw push URLs.

## Attempt 1: requested default command

```bash
mkdir -p target/runtime-proof && set -o pipefail; \
  nix run .#dogfood-local -- full \
  2>&1 | tee target/runtime-proof/dogfood-full.log
```

Result: exit 1.

Receipt: `/tmp/aspen-dogfood-receipts/dogfood-20260513T034410Z.json`.

Receipt stages:

- `start`: succeeded; single-node cluster reached running state; ticket redacted in log.
- `push`: failed after 1007 ms.
- `stop`: succeeded.

Failure message:

```text
forge create repo: KV storage error: not leader; current leader: None; when Write Logs: std::io::error::Error: disk usage too high: 98% (threshold: 95%)
```

Host disk probe immediately after failure:

```text
/dev/nvme1n1p3  1.8T  1.6T  123G  93% /
datapool/nix    2.2T  331G  1.9T   15% /nix
tmpfs            94G   92G  1.5G   99% /tmp
```

Classification: default dogfood cluster directory `/tmp/aspen-dogfood` is blocked on host `/tmp` pressure. This is host/environment capacity, not proof of dogfood product failure.

## Attempt 2: same app with cluster directory on datapool-backed path

```bash
mkdir -p target/runtime-proof && set -o pipefail; \
  nix run .#dogfood-local -- --cluster-dir /home/brittonr/data/aspen-dogfood-proof full \
  2>&1 | tee target/runtime-proof/dogfood-full-datapool.log
```

Result: exit 1.

Receipt: `/home/brittonr/data/aspen-dogfood-proof-receipts/dogfood-20260513T034555Z.json`.

Receipt stages:

- `start`: succeeded in 10555 ms.
- `push`: failed after 602059 ms.
- `stop`: succeeded in 2001 ms.

Failure message:

```text
git push aspen-dogfood timed out after 600s
```

Process inspection during the timeout showed `git push aspen-dogfood HEAD:refs/heads/main --force`, `git-remote-aspen`, and the repository pre-push hook (`pre-commit ... hook-type=pre-push`) still resident under the dogfood process.

Classification: with the cluster directory moved off `/tmp`, Aspen reached cluster start, Forge repo creation, CI watch registration, and the beginning of `git push`. The run did not reach build/deploy/verify/self-hosting acceptance because the push stage timed out at the local git/pre-push boundary. This is classified as a workflow/host gate blocker pending either hook-safe dogfood push semantics or a longer/explicitly managed push gate; no final dogfood acceptance receipt was produced.
