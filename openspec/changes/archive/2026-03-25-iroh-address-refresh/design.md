## Context

The address lifecycle today:

```
startup → iroh::Endpoint::bind(0)          → new random port
        → SecretKey::from_file()           → same endpoint ID
        → gossip::broadcast_announcement() → announces new addr to gossip peers
        → Raft state machine               → still has old addr from add_learner

new_client(target, RaftMemberInfo):
  1. addr = node.iroh_addr  ← stale, from Raft membership
  2. peer_addrs[target] = addr  ← caches the stale addr
  3. connect(addr)  ← fails, port changed
```

The gossip layer already discovers the new address and calls `factory.add_peer()`, which writes to `peer_addrs`. But `new_client()` reads from `RaftMemberInfo` first and ignores the cache.

## Goals / Non-Goals

**Goals:**

- Restarted nodes rejoin the cluster within seconds (1-2 heartbeat cycles)
- Zero manual intervention after restart
- Works for all restart scenarios: systemctl restart, deploy, crash, network change
- No new RPC types or protocol changes

**Non-Goals:**

- Handling endpoint ID changes (secret key rotation). That requires full membership change.
- Guaranteeing the Raft membership state is always up-to-date (gossip override is sufficient for connectivity; membership sync is a nice-to-have)
- Discovery of entirely new nodes (that's what `add_learner` is for)

## Decisions

### 1. Gossip cache takes priority over Raft membership when endpoint IDs match

In `new_client()`, check `peer_addrs` first. If the gossip cache has an entry for the target node with the **same endpoint ID** but **different socket addresses**, use the gossip address. This is safe because:

- Same endpoint ID = same secret key = same node identity (not a MITM)
- Gossip announcements are cryptographically signed
- The cache is populated by verified `PeerAnnouncement` messages

```
new_client(target, RaftMemberInfo):
  1. gossip_addr = peer_addrs.get(target)
  2. if gossip_addr.id == node.iroh_addr.id && gossip_addr.addrs != node.iroh_addr.addrs:
       addr = gossip_addr  ← fresher address from gossip
       log "using gossip-discovered address for node {target}"
  3. else:
       addr = node.iroh_addr  ← original from Raft membership
  4. connect(addr)
```

### 2. Eager announcement after startup

Today, gossip announcements run on a periodic timer. After a restart, the first announcement might not fire for 30-60s. Add an immediate `broadcast_announcement()` call after gossip setup completes in the bootstrap sequence. This gets the new address to peers within ~1 RTT.

### 3. No Raft membership sync (for now)

Updating `RaftMemberInfo` in the Raft state requires a new `add_learner` call or a custom write command. Both are heavyweight (consensus round) and risk disrupting the cluster during restart recovery. The gossip override is sufficient — it's fast, decentralized, and doesn't require leader availability.

If we add membership sync later, it should be a leader-only background task that periodically compares gossip-discovered addresses with Raft membership and issues non-disruptive updates.

## Risks / Trade-offs

- **[Split-brain risk: None]** The endpoint ID check ensures we only accept address updates for the same cryptographic identity. An attacker would need the secret key.
- **[Stale gossip cache]** If gossip cache has an even older address (from a previous restart), we'd pick the wrong one. Mitigated by: gossip announcements include timestamps, and the cache is populated by the most recent announcement. We can add a recency check.
- **[Raft membership drift]** The Raft state machine will have stale addresses forever (until membership change). This is cosmetic — `cluster status` will show old addresses. Acceptable for now; membership sync can fix later.
