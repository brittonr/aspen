## Context

The signed two-node VM fixture has an independently selected (VM-local) signed startup output, but its storage-denial case is unavailable. On an initialized and stopped node, `node status` reads startup state, then its first mutation is `NodeStateRoot::write` of `health-receipt.preserves`. A missing health file under an owned, mode-0500 root yields a kernel `EACCES` at `open_with(O_CREAT|O_TRUNC)` when executed as the non-root service user without DAC-override capabilities. The current generic I/O conversion retains only text, and the VM receipt validator sees only caller-supplied refs.

## Decisions

### Decision: Observe the existing status write, not a storage probe

**Choice:** Run the selected binary's ordinary `node status` under the actual service identity against node_b's protected root. Preserve a typed failure only from the `open_with` syscall before a file handle exists; write/flush failures remain generic and cannot qualify no-mutation. Normal status still exits nonzero. A dedicated optional failure-receipt output outside the root serializes the typed failure and actual effective UID, GID, groups, capabilities and current executable identity. It does not independently authorize the operation.

**Rationale:** Status has an established protected write before other writes. `node init` may create layout and identity state before its final config write, making the no-mutation assertion harder to establish.

### Decision: Independently observe protected state

**Choice:** A separate read-only VM observer, invoked by the root test driver before and after the non-root status process, records bounded sorted relative entries, content hashes, ownership/mode and the absent target in canonical observations written outside the protected root. The fault configuration observation captures actual mode/owner, not a hand-authored injection marker. Both observations bind selected executable/build, startup binding and node config, and their protected-tree identities must match.

**Rationale:** A process asserting its own lack of mutation, or two refs to one old node evidence file, is insufficient. Observations must originate from separate actual filesystem reads on either side of the attempt.

### Decision: Qualify only linked canonical artifacts

**Choice:** Keep the current fault descriptor and receipt schemas, but require the `fault-validate` input to contain the actual typed attempt, pre/post and permission observations. Hash those values and require exact matches to the receipt's child/pre/post/injection refs. Check target node, operation, executable/build/binding/config, identity, effective permissions, EACCES/open-before-write, target absence and equal protected-tree identity. If any observation/namespace capability is unavailable, retain `decision=unavailable` and no observed-denial status. Synthetic structural fixtures remain valid as unavailable fixtures.

**Rationale:** A nonempty child ref, log or caller-selected decision cannot prove execution. The validator must inspect artifact content and linkage.

### Decision: Bound the whole qualification campaign separately from startup

The first retained signed run exhausted the test driver's 3,600-second aggregate budget while waiting for initial service startup; it produced no storage-denial verdict. The next declared profile uses 8 vCPUs and 8 GiB per VM and a four-hour aggregate campaign horizon. Per-startup service/wait deadlines remain 3,600 seconds and all source/authority/fault predicates remain unchanged. This is an explicit fixture-resource profile, not a relaxed product deadline, passing execution, producer/operator independence or production latency qualification. Preserve the failed observation and require the actual new profile's result before closeout.

The 8-vCPU/8-GiB signed attempt also failed the individual 3,600-second startup wait, before integration assertions. The next profile uses NixOS's `virtualisation.useNixStoreImage` for a complete local read-only store instead of the host-store 9p transport. Exact store paths, retained bytes, signature checks, source-closure measurement, operation deadlines and all fault predicates remain unchanged. The pinned NixOS option recommends this backing for read-heavy workloads; the failed run does not prove a transport root cause, and only actual new execution can establish an outcome.

## Risks / Trade-offs

- A permission error after a successful file open could follow a partial mutation; only the typed failed-open phase qualifies.
- Root-created receipts cannot prove independent producer/operator trust. The VM-local signer and the root test driver establish this fixture's mechanism only.
- The VM image must support running as the service UID without DAC-override and bounded read-only filesystem observations. If not, the result is unavailable, never a fabricated denial.
