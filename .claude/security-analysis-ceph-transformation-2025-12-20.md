# Security Analysis: Aspen to Ceph-like Storage System Transformation

**Date**: 2025-12-20
**Purpose**: Comprehensive security threat analysis for the proposed Ceph-like storage transformation

## Executive Summary

The plan to transform Aspen from an etcd-like coordination system into a Ceph-like distributed storage platform introduces **significant security attack surface expansion**. Current Aspen has ~350 lines of authentication code (HMAC-SHA256 for Raft RPC). The Ceph-like plan requires implementing **6 major security domains** with minimal detail on authentication, authorization, encryption, and DoS protection.

### Critical Security Gaps Identified

1. **Authentication Gaps**: OSD-to-OSD, client-to-OSD, MDS authentication undefined
2. **Authorization Model**: No design for S3 ACLs, bucket policies, or MDS capabilities
3. **Encryption**: Key management undefined, metadata encryption missing
4. **Trust Boundaries**: New untrusted zones (clients, OSDs) with no security model
5. **DoS Resistance**: No rate limiting for S3 gateway, OSD recovery bandwidth
6. **Supply Chain**: Unknown security audit status for s3s, nbd crates

**Recommendation**: The plan significantly underestimates security implementation effort. Add 2-4 months per phase for security hardening.

---

## 1. Authentication and Authorization Gaps

### 1.1 Current Aspen Authentication (Implemented)

Aspen has **production-ready HMAC-SHA256 authentication** for Raft RPC:

**File**: `/home/brittonr/git/aspen/src/raft/auth.rs` (429 lines)

```rust
/// Authentication challenge sent by the server.
pub struct AuthChallenge {
    pub nonce: [u8; AUTH_NONCE_SIZE],  // 32 bytes
    pub timestamp_ms: u64,
    pub protocol_version: u8,
}

/// Authentication response sent by the client.
pub struct AuthResponse {
    pub hmac: [u8; AUTH_HMAC_SIZE],  // HMAC-SHA256 (32 bytes)
    pub client_nonce: [u8; AUTH_NONCE_SIZE],
}
```

**Authentication Flow (Implemented)**:

1. Server generates random nonce and timestamp
2. Client computes `HMAC-SHA256(cookie_key, nonce || timestamp || client_endpoint_id)`
3. Server verifies HMAC with constant-time comparison
4. Both parties mutually authenticated

**Security Features**:

- Fixed nonce size (32 bytes = 256 bits entropy)
- Challenge expiration (60 seconds max age)
- Constant-time HMAC comparison (timing attack resistant)
- Mutual authentication (prevents impersonation)
- Blake3-derived keys from cluster cookie
- Tiger Style: All message sizes bounded (256 bytes max)

**Scope**: Currently only covers Raft cluster membership (monitor-to-monitor in Ceph terms).

### 1.2 Missing Authentication in Ceph-like Plan

The plan mentions "HMAC-SHA256 for Raft RPC (already implemented)" but fails to define authentication for **5 critical trust boundaries**:

#### A. OSD-to-OSD Authentication (MISSING)

**Plan Reference** (Phase 2: OSD, line 906-932):

```
OSD Architecture:
├── Object Storage Engine
├── PG Management
│   ├── Primary handling (client writes)
│   ├── Replica handling (replication from primary)
│   └── Recovery handling (data repair)
```

**Security Gap**: No authentication defined for:

- Primary OSD → Secondary OSD replication
- OSD → OSD recovery traffic
- OSD heartbeats to monitor

**Comparison to Ceph**: Ceph uses **CephX** (Kerberos-like) for all daemon-to-daemon authentication:

- OSDs authenticate to monitors using shared secrets
- OSDs verify peer OSD identity before accepting replication
- Session keys expire, requiring re-authentication
- Mutual authentication prevents Byzantine OSDs

**Threat Without Authentication**:

- **Byzantine OSD Attack**: Malicious OSD joins cluster, corrupts replica data
- **Man-in-the-Middle**: Attacker intercepts replication, modifies objects
- **Data Exfiltration**: Rogue OSD requests recovery, receives all PG data

**Required Implementation**:

```rust
// src/osd/auth.rs (NEW FILE - ~500 lines)
pub struct OsdAuthContext {
    cluster_cookie: Vec<u8>,
    osd_id: OsdId,
    session_keys: RwLock<HashMap<OsdId, SessionKey>>,
}

pub struct SessionKey {
    key: [u8; 32],
    expires_at: u64,
}

impl OsdAuthContext {
    /// Authenticate peer OSD for replication
    async fn authenticate_peer(&self, peer_id: OsdId) -> Result<SessionKey>;

    /// Verify replication request HMAC
    fn verify_replication_hmac(&self, request: &ReplicationRequest) -> bool;
}
```

**Effort Estimate**: 2-3 weeks (auth protocol design + implementation + tests)

#### B. Client-to-OSD Authentication (MISSING)

**Plan Reference** (Phase 4: S3 Gateway, line 1536-1542):

```
S3 Gateway Architecture:
├── HTTP/HTTPS Layer (axum with Tower)
├── S3 Protocol Handler (using s3s crate)
├── Auth Layer (S3Auth)
│   └── HMAC-SHA256 signature verification
```

**Security Gap**: Plan mentions "S3Auth" but provides **zero implementation detail**:

- How are S3 access keys generated and stored?
- Where are S3 secret keys persisted (Raft KV? External KMS?)
- How does S3 gateway obtain OSD locations (through monitor)?
- Does the gateway authenticate to OSDs on behalf of clients?

**Comparison to Ceph**: Ceph uses CephX for client authentication:

1. Client authenticates to monitor with shared secret
2. Monitor issues ticket for accessing OSDs
3. Client presents ticket to OSD for I/O operations
4. Tickets expire (default: 1 hour)

**S3 Authentication Complexity**: AWS S3 uses **Signature Version 4**:

1. Client sends HMAC-SHA256 signature with each request
2. Server computes expected signature using secret key
3. Signature includes: HTTP method, URI, headers, payload hash
4. Prevents replay attacks with timestamp validation

**Implementation Gap**:

```rust
// How S3Auth would need to work (UNDEFINED in plan):

pub struct S3AuthLayer {
    // Where are these stored?
    access_keys: HashMap<String, SecretKey>,

    // How do we verify signatures?
    async fn verify_signature(&self, request: &S3Request) -> Result<AccountId>;

    // How do we map S3 accounts to OSD access?
    async fn authorize_osd_access(&self, account: AccountId, object: &str) -> Result<OsdTicket>;
}
```

**Threat Without Authentication**:

- **Unauthenticated Object Access**: Anyone can read/write objects
- **Resource Exhaustion**: Attackers fill storage with garbage data
- **Data Deletion**: Malicious clients delete all buckets

**Effort Estimate**: 3-4 weeks (S3 SigV4 implementation + credential store + OSD ticket system)

#### C. MDS Authentication (MISSING)

**Plan Reference** (Phase 5: MDS, line 1614-1631):

```
MDS Architecture:
├── FUSE / VirtioFS Client
├── MDS Protocol Handler (Directory ops, Inode management, Locking)
├── Metadata Server Cluster (Raft-based consensus)
```

**Security Gap**: No authentication defined for:

- FUSE client → MDS authentication
- How are client credentials verified?
- How are POSIX permissions enforced?

**Comparison to Ceph**: CephFS uses **capabilities**:

1. Client authenticates to monitor with CephX
2. Monitor issues MDS capability token
3. Token specifies allowed paths and operations
4. MDS verifies capability on each operation

**POSIX Permission Problem**: FUSE passes UID/GID from client kernel:

```rust
// FUSE provides these - but can we trust them?
pub struct FuseRequest {
    pub uid: u32,  // Client claims to be this user
    pub gid: u32,  // Client claims to be in this group
}
```

**Attack**: Client spoofs UID=0 (root) to gain privileged access.

**Required Implementation**:

```rust
// src/mds/capabilities.rs (NEW FILE - ~400 lines)
pub struct MdsCapability {
    client_id: ClientId,
    allowed_paths: Vec<PathPrefix>,
    allowed_ops: Operations,  // READ, WRITE, DELETE
    expires_at: u64,
}

impl MetadataServer {
    /// Verify client capability before operation
    async fn check_capability(&self, cap: &MdsCapability, path: &str, op: Operation) -> Result<()>;

    /// Issue new capability after authentication
    async fn issue_capability(&self, client_id: ClientId) -> Result<MdsCapability>;
}
```

**Effort Estimate**: 2-3 weeks (capability system + FUSE integration)

### 1.3 Authorization Model (UNDEFINED)

The plan mentions "Per-bucket/object ACLs for S3" (line 1795) but provides **no design**:

#### S3 Authorization Complexity

AWS S3 has **3 layers of authorization**:

1. **IAM Policies**: Account-level permissions
2. **Bucket Policies**: Resource-based access control (JSON)
3. **ACLs**: Per-object access control (legacy, XML-based)

**Example Bucket Policy** (prevents public writes):

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Deny",
      "Principal": "*",
      "Action": "s3:PutObject",
      "Resource": "arn:aws:s3:::my-bucket/*",
      "Condition": {
        "StringNotEquals": {
          "s3:x-amz-server-side-encryption": "AES256"
        }
      }
    }
  ]
}
```

**Implementation Requirements**:

1. Policy storage (Raft KV for bucket policies)
2. Policy evaluation engine (condition matching)
3. Principal resolution (map S3 access keys to accounts)
4. Action authorization (check policy before OSD access)

**Missing from Plan**:

- Where are bucket policies stored? (Raft KV? SQLite?)
- How are policies evaluated? (Policy engine implementation?)
- How are cross-account permissions handled?
- What about pre-signed URLs? (Time-limited public access)

**Comparison to Ceph**: Ceph RGW supports:

- Bucket policies (full S3 compatibility)
- ACLs (per-object, discouraged)
- User quotas (max buckets, max objects, max size)

**Effort Estimate**: 4-6 weeks (policy storage + evaluation engine + S3 compatibility tests)

---

## 2. Trust Boundaries and Security Perimeters

### 2.1 Current Aspen Trust Model (Simple)

```
┌─────────────────────────────────────────────────────────────┐
│                    Raft Cluster (Trusted)                    │
│  All nodes share cluster cookie, mutually authenticated     │
│  ├─ Node 1 (Raft voter)                                     │
│  ├─ Node 2 (Raft voter)                                     │
│  └─ Node 3 (Raft learner)                                   │
└─────────────────────────────────────────────────────────────┘
         ↑
         │ Authenticated Iroh QUIC (HMAC-SHA256)
         │
    Client (Untrusted)
```

**Trust Boundary**: Single boundary between cluster (trusted) and clients (untrusted).

### 2.2 Ceph-like Trust Model (Complex)

The plan introduces **4 new trust zones**:

```
┌─────────────────────────────────────────────────────────────┐
│              Monitor Cluster (Trusted Coordinator)           │
│  - Manages cluster topology (CRUSH map)                     │
│  - Issues authentication tickets                            │
│  - Stores encryption keys                                   │
└─────────────────────────────────────────────────────────────┘
         ↑                    ↑                    ↑
         │ OSD Auth?          │ Client Auth?       │ MDS Auth?
         │                    │                    │
┌────────┴─────┐     ┌───────┴────────┐     ┌────┴────────────┐
│ OSD Cluster  │     │  S3 Gateway    │     │  MDS Cluster    │
│ (Semi-Trust) │     │  (Untrusted    │     │  (Semi-Trust)   │
│              │     │   Clients)     │     │                 │
└──────────────┘     └────────────────┘     └─────────────────┘
```

**New Trust Boundaries**:

1. **Monitor ↔ OSD**: OSDs must prove identity to monitor
2. **OSD ↔ OSD**: Replicas must verify peer identity
3. **Client ↔ S3 Gateway**: S3 SigV4 authentication
4. **S3 Gateway ↔ OSD**: Gateway must authenticate to OSD
5. **FUSE Client ↔ MDS**: Client must prove identity
6. **MDS ↔ OSD**: MDS reads file data from OSDs

**Security Implications**:

- **6 authentication protocols** vs. current 1
- **Byzantine fault tolerance**: OSDs could be malicious
- **Multi-tenant isolation**: S3 accounts must not see each other's data

**Plan's Treatment**: "Authentication: HMAC-SHA256 for Raft RPC (already implemented)" (line 1794)

**Reality**: Existing auth only covers Monitor ↔ Monitor. **5 trust boundaries undefined**.

---

## 3. Data at Rest Encryption (UNDEFINED)

### 3.1 Plan's Encryption Coverage

**Plan Reference** (line 1797):

```
Encryption: TLS for all network traffic (via Iroh QUIC)
Data at rest: Optional encryption for blob storage
```

**Analysis**: Plan provides **zero detail** on:

- How is "optional encryption" configured?
- Where are encryption keys stored?
- How are keys rotated?
- What about metadata encryption?

### 3.2 Ceph's Encryption Architecture

Ceph supports **2 types of encryption at rest**:

#### A. OSD Encryption (Full Disk)

- Uses **LUKS** (Linux Unified Key Setup) with dmcrypt
- Encrypts entire OSD logical volume (LVM)
- Keys stored in **Monitor keyring** (not on OSD disk)
- OSD requests decryption key from monitor on startup

**Key Management Flow**:

1. OSD submits authentication to monitor
2. Monitor verifies OSD identity (CephX)
3. Monitor returns LUKS decryption key
4. OSD unlocks encrypted volume
5. Key remains in memory, not persisted

#### B. RGW Encryption (Object-Level)

- **Server-Side Encryption (SSE)**: Objects encrypted by RGW
- **SSE-C**: Customer-provided keys (key in HTTP header)
- **SSE-KMS**: Keys managed by Vault or AWS KMS
- Metadata is **not encrypted** (object names, sizes visible)

### 3.3 Aspen's Encryption Gaps

The plan uses **iroh-blobs** for object storage:

**iroh-blobs**: Content-addressed blob storage using **Blake3 hashes**.

**Problem**: iroh-blobs has **no built-in encryption**. It stores blobs as:

```
data_dir/blobs/{hash}.blob  (plaintext)
```

**Required Implementation**:

```rust
// src/osd/encryption.rs (NEW FILE - ~600 lines)

pub enum EncryptionMode {
    None,
    Aes256Gcm { key_id: KeyId },
    ChaCha20Poly1305 { key_id: KeyId },
}

pub struct EncryptedBlobStore {
    inner: IrohBlobStore,
    kms: Box<dyn KeyManagementService>,
}

impl EncryptedBlobStore {
    /// Encrypt blob before writing to iroh-blobs
    async fn write_encrypted(&self, data: Bytes, key_id: KeyId) -> Result<Hash>;

    /// Decrypt blob after reading from iroh-blobs
    async fn read_decrypted(&self, hash: Hash, key_id: KeyId) -> Result<Bytes>;
}

/// Key Management Service abstraction
#[async_trait]
pub trait KeyManagementService: Send + Sync {
    /// Fetch encryption key by ID
    async fn get_key(&self, key_id: KeyId) -> Result<EncryptionKey>;

    /// Rotate keys (generate new, deprecate old)
    async fn rotate_key(&self, key_id: KeyId) -> Result<KeyId>;

    /// Secure key deletion
    async fn delete_key(&self, key_id: KeyId) -> Result<()>;
}
```

**Key Management Options**:

1. **Embedded** (keys in Raft KV): Simple but single point of failure
2. **HashiCorp Vault**: Industry standard, complex integration
3. **AWS KMS / GCP KMS**: Cloud-specific, vendor lock-in

**Metadata Encryption Gap**:

- Raft SQLite state machine stores **object metadata unencrypted**
- Attack: Adversary with disk access learns object names, sizes, access patterns

**Example Metadata Leak**:

```sql
-- SQLite state machine (UNENCRYPTED)
SELECT key, value_size FROM kv_store WHERE key LIKE '/bucket/sensitive-project/%';
```

Reveals:

- Object keys (file paths)
- Object sizes
- Creation timestamps
- Access patterns

**Effort Estimate**: 6-8 weeks (encryption layer + KMS integration + key rotation + tests)

---

## 4. Network Security and DoS Protection

### 4.1 Current Aspen DoS Protections (Implemented)

Aspen has **production-ready Tiger Style limits** (`/home/brittonr/git/aspen/src/raft/constants.rs`):

```rust
// Network DoS protection
pub const MAX_RPC_MESSAGE_SIZE: u32 = 10 * 1024 * 1024;  // 10 MB
pub const MAX_SNAPSHOT_SIZE: u64 = 100 * 1024 * 1024;    // 100 MB
pub const MAX_CONCURRENT_CONNECTIONS: u32 = 500;
pub const MAX_STREAMS_PER_CONNECTION: u32 = 100;
pub const MAX_PEERS: u32 = 1000;

// Storage DoS protection
pub const MAX_KEY_SIZE: u32 = 1024;          // 1 KB
pub const MAX_VALUE_SIZE: u32 = 1024 * 1024; // 1 MB
pub const MAX_BATCH_SIZE: u32 = 1000;

// Gossip DoS protection (line 368-406)
pub const GOSSIP_PER_PEER_RATE_PER_MINUTE: u32 = 12;
pub const GOSSIP_PER_PEER_BURST: u32 = 3;
pub const GOSSIP_GLOBAL_RATE_PER_MINUTE: u32 = 10_000;
```

**Rate Limiting Implementation** (`src/cluster/gossip_discovery.rs`):

```rust
struct GossipRateLimiter {
    per_peer_limiter: LruCache<PublicKey, LeakyBucket>,
    global_limiter: LeakyBucket,
}

fn check(&mut self, peer_id: &PublicKey) -> Result<(), RateLimitReason> {
    if !self.global_limiter.try_acquire() {
        return Err(RateLimitReason::Global);
    }

    let bucket = self.per_peer_limiter.get_or_insert(peer_id.clone(), || {
        LeakyBucket::new(GOSSIP_PER_PEER_RATE_PER_MINUTE, GOSSIP_PER_PEER_BURST)
    });

    if !bucket.try_acquire() {
        return Err(RateLimitReason::PerPeer);
    }

    Ok(())
}
```

**Scope**: Only covers gossip discovery. Raft RPC has connection limits but **no rate limiting**.

### 4.2 Ceph-like DoS Vectors (UNDEFINED in Plan)

The plan introduces **4 major DoS attack surfaces**:

#### A. S3 Gateway DoS (NO RATE LIMITING)

**Attack Vectors**:

1. **Slowloris**: Open many connections, send headers slowly
2. **Large multipart uploads**: Upload 10,000 parts × 5GB = 50TB
3. **LIST requests**: List 1M objects repeatedly (expensive SQL queries)
4. **DELETE floods**: Delete-recreate objects in tight loop

**Plan Reference** (line 1542-1596): **Zero mention of rate limiting**.

**Required Implementation**:

```rust
// src/s3/rate_limit.rs (NEW FILE - ~400 lines)

pub struct S3RateLimiter {
    // Per-IP rate limits
    ip_limiters: LruCache<IpAddr, TokenBucket>,

    // Per-account rate limits
    account_limiters: HashMap<AccountId, TokenBucket>,

    // Global cluster-wide limits
    global_limiter: TokenBucket,
}

/// Tiger Style limits for S3 gateway
pub const S3_MAX_REQUESTS_PER_SECOND_PER_IP: u32 = 100;
pub const S3_MAX_REQUESTS_PER_SECOND_PER_ACCOUNT: u32 = 1000;
pub const S3_MAX_BANDWIDTH_MBPS_PER_IP: u32 = 100;
pub const S3_MAX_MULTIPART_PARTS: u32 = 10_000;
pub const S3_MAX_LIST_RESULTS: u32 = 1000;
```

**Comparison to AWS S3**: AWS has extensive rate limiting:

- 3,500 PUT/COPY/POST/DELETE per second per prefix
- 5,500 GET/HEAD per second per prefix
- Auto-scaling for higher rates (but costs $$)

**Ceph RGW**: Configurable rate limits per user:

```
rgw_user_ratelimit_bucket = 100      # Bucket ops/min
rgw_user_ratelimit_object = 1000     # Object ops/min
rgw_user_bandwidth_quota_max = 1 GB  # Bandwidth/user
```

**Effort Estimate**: 3-4 weeks (rate limiter + integration + tests)

#### B. OSD Recovery DoS (UNBOUNDED BANDWIDTH)

**Plan Reference** (Phase 2, line 1173-1178):

```rust
/// Recovery loop - handle PG recovery.
async fn recovery_loop(&self) {
    // Recovery implementation
}
```

**Attack Scenario**:

1. Attacker crashes OSD
2. Recovery starts for all PGs on that OSD
3. Recovery saturates network bandwidth
4. Client I/O starves
5. Cluster becomes unusable

**Ceph's Solution**: **Recovery throttling**:

```
osd_recovery_max_active = 3           # Max concurrent recoveries
osd_recovery_sleep_hdd = 0.1          # Sleep between recovery I/O (HDD)
osd_max_backfills = 1                 # Max backfill operations
osd_recovery_max_single_start = 1    # Start 1 recovery at a time
```

**Required Implementation**:

```rust
// src/osd/recovery.rs

pub struct RecoveryThrottler {
    max_concurrent: u32,        // Tiger Style: 10 max
    active_recoveries: AtomicU32,
    bandwidth_limiter: TokenBucket,
}

pub const MAX_CONCURRENT_RECOVERIES: u32 = 10;
pub const RECOVERY_BANDWIDTH_MBPS: u32 = 100;  // Max recovery bandwidth
pub const RECOVERY_SLEEP_MS: u64 = 10;         // Sleep between chunks
```

**Effort Estimate**: 2-3 weeks (throttling + bandwidth limiting + tests)

#### C. MDS DoS (NO LIMITS ON DIRECTORY OPERATIONS)

**Plan Reference** (Phase 5, line 1732-1734):

```rust
async fn readdir(&self, ino: Ino, offset: u64, limit: u32) -> Result<Vec<DirEntry>>;
```

**Attack Vectors**:

1. **Directory listing**: `ls` on directory with 1M files
2. **Inode exhaustion**: Create 1B empty files
3. **Lock thrashing**: Rapidly lock/unlock files

**Required Limits**:

```rust
pub const MAX_DIR_ENTRIES: u32 = 1_000_000;  // Mentioned in plan (line 1695)
pub const MAX_READDIR_RESULTS: u32 = 10_000; // But not enforced anywhere
pub const MAX_LOCKS_PER_FILE: u32 = 1000;    // Mentioned but not implemented
```

**Effort Estimate**: 2 weeks (MDS resource limits + tests)

#### D. CRUSH Calculation DoS (CPU EXHAUSTION)

**Plan Reference** (Phase 1, line 630-678): CRUSH algorithm implementation

**Attack**: Malicious client repeatedly queries placement for 1M objects:

```rust
for i in 0..1_000_000 {
    let placement = crush_map.crush(pg_id, "replicated_rule", 3)?;
}
```

**Mitigation**: Cache CRUSH results per PG:

```rust
pub struct CrushCache {
    cache: LruCache<(u32, String), PlacementResult>,
}

pub const CRUSH_CACHE_SIZE: usize = 10_000;  // Cache 10k PGs
```

**Effort Estimate**: 1 week (CRUSH caching + benchmarks)

---

## 5. Supply Chain Security Analysis

### 5.1 Plan's New Dependencies

The plan introduces **2 major external crates**:

#### A. `s3s` Crate (S3 Server Implementation)

**Plan Reference** (line 1551, 1561, 1835):

```rust
use s3s::auth::S3Auth;
use s3s::S3;
```

**Crate Analysis**:

- **GitHub**: https://github.com/Nugine/s3s
- **Crates.io**: https://crates.io/crates/s3s
- **Maintainer**: Single individual (Nugine)
- **Downloads**: ~50,000 total (low adoption)
- **Last Release**: Unknown (search didn't find version info)
- **Security Audit**: **NONE FOUND** in public records

**Security Concerns**:

1. **Single maintainer**: Bus factor = 1
2. **Low adoption**: Less scrutiny than AWS SDK
3. **No security audit**: Unverified S3 API implementation
4. **Attack Surface**: Parses untrusted S3 requests (XML, JSON)

**Comparison to Alternatives**:

- **MinIO SDK**: Used by MinIO (production-proven, multi-maintainer)
- **AWS SDK for Rust**: Official AWS, heavily audited
- **Custom implementation**: Full control, high effort

**Recommendation**: **Conduct security audit of s3s crate** before use, or implement custom S3 subset.

#### B. `nbd` Crate (Network Block Device)

**Plan Reference** (line 1417-1507): NBD server for block devices

**Crate Search**: Likely `rust-nbd` or similar (plan doesn't specify)

**Security Concerns**:

1. **Kernel interface**: NBD talks directly to Linux kernel
2. **Block device access**: Errors can corrupt filesystems
3. **Privilege escalation**: NBD runs with high privileges

**Recommendation**: Thoroughly vet NBD crate for memory safety and protocol compliance.

### 5.2 RustSec Advisory Database Status

**Tools**: `cargo audit` checks for known vulnerabilities in dependencies.

**Current Aspen Dependencies** (from existing codebase):

- openraft: **Vendored** (custom fork, not audited by RustSec)
- iroh: Large crate (~100k LoC), no known CVEs
- redb: Embedded database, no known CVEs
- rusqlite: SQLite wrapper, no known CVEs

**New Dependencies** (from plan):

- s3s: **Not in RustSec** (too new or low adoption)
- nbd: **Check RustSec** before use

**Action Items**:

1. Run `cargo audit` on final dependency tree
2. Enable Dependabot for automated CVE alerts
3. Subscribe to RustSec advisory feed

---

## 6. Comparison: Ceph's Security Model

### 6.1 CephX Authentication Protocol

Ceph uses **CephX** (Kerberos-like authentication):

**Components**:

1. **Monitors**: Act as Key Distribution Center (KDC)
2. **Shared Secrets**: Each entity has secret key stored in monitor
3. **Session Tickets**: Short-lived credentials for accessing services
4. **Capabilities**: Fine-grained permissions (mon, osd, mds scopes)

**Authentication Flow**:

```
Client                Monitor                 OSD
  |                      |                      |
  |-- Auth Request ----> |                      |
  |   (using secret)     |                      |
  |                      |                      |
  | <--- Ticket -------- |                      |
  |   (session key)      |                      |
  |                      |                      |
  |------------ Read Request -----------------> |
  |   (with ticket)                             |
  |                                             |
  | <------------ Data ----------------------- |
```

**Ticket Format** (simplified):

```rust
pub struct CephTicket {
    client_id: String,
    service: ServiceType,  // mon, osd, mds
    session_key: [u8; 32],
    expires_at: u64,
    capabilities: Vec<Capability>,
}

pub struct Capability {
    service: ServiceType,
    allowed_ops: Operations,  // r, w, x
    pool: Option<PoolId>,     // Restrict to specific pool
}
```

**Aspen Equivalent**: **NOT DEFINED** in plan.

**Required for Ceph-like system**:

1. Ticket issuance by monitors
2. Ticket verification by OSDs
3. Capability-based authorization
4. Session key renewal

**Effort Estimate**: 4-6 weeks (full CephX-like protocol)

### 6.2 Ceph Encryption Support

#### Network Encryption (msgr v2)

**Ceph msgr v2** (introduced in Nautilus 14.2.0):

- **TLS-like encryption**: AES-128-GCM or ChaCha20-Poly1305
- **Forward secrecy**: ECDHE key exchange
- **Authentication**: Integrated with CephX
- **Overhead**: ~5-10% CPU, negligible latency

**Aspen Equivalent**: **Iroh QUIC** provides similar features:

- TLS 1.3 encryption (AES-128-GCM or ChaCha20)
- Forward secrecy (X25519 ECDHE)
- 0-RTT connection resumption

**Status**: ✅ Network encryption already implemented via Iroh.

#### Data-at-Rest Encryption

**Ceph OSD Encryption**:

- LUKS/dmcrypt for full disk encryption
- Keys stored in monitor keyring (not on OSD disk)
- OSD authenticates to monitor on boot, receives key
- **Limitation**: Metadata not encrypted (object names visible)

**RGW Server-Side Encryption**:

- SSE-C: Customer-provided keys (ephemeral, not stored)
- SSE-KMS: Keys from Vault or AWS KMS
- Per-object encryption (different keys per object)
- Metadata **not encrypted** (sizes, timestamps visible)

**Aspen Status**: ❌ **Data-at-rest encryption undefined in plan**.

### 6.3 Ceph Known Vulnerabilities (2024-2025)

Based on CVE search, Ceph has had **serious security issues**:

#### CVE-2025 (Unreleased): CephFS Privilege Escalation

**Impact**: Unprivileged user can escalate to root in ceph-fuse mount
**Mechanism**: `chmod 777` on root-owned directory grants access
**Affected Versions**: 17.2.7, 18.2.1-18.2.4, 19.0.0-19.2.2
**Fixed**: 17.2.8, 18.2.5, 19.2.3
**Severity**: **CRITICAL** (confidentiality, integrity, availability)

**Lesson for Aspen**: FUSE permission checking is complex and error-prone.

#### CVE-2025: JWT Authentication Bypass (RGW)

**Impact**: Attacker can bypass OIDC authentication
**Mechanism**: JWT with `alg: "none"` not validated
**Affected Versions**: All versions ≤ 19.2.3
**Fixed**: **NOT YET PATCHED**
**Severity**: **CRITICAL** (authentication bypass)

**Lesson for Aspen**: Third-party auth integration (OIDC, JWT) requires expert review.

#### CVE-2025: RGW DoS via Empty Copy-Source

**Impact**: RGW daemon crashes, causing DoS
**Mechanism**: `x-amz-copy-source: ""` crashes daemon
**Affected Versions**: All versions ≤ 19.2.3
**Fixed**: **NOT YET PATCHED**
**Severity**: **HIGH** (availability)

**Lesson for Aspen**: S3 API has many edge cases; thorough input validation required.

#### CVE-2024-1313: Grafana Authorization Bypass

**Impact**: User in different org can delete snapshots
**Severity**: **MEDIUM**

**Lesson for Aspen**: Third-party dependencies (Grafana, s3s) introduce vulnerabilities.

---

## 7. Threat Modeling: Attack Scenarios

### 7.1 Byzantine OSD Attack

**Threat Actor**: Malicious insider or compromised OSD node

**Attack Steps**:

1. Attacker deploys rogue OSD with valid cluster cookie
2. OSD registers with monitor cluster (no authentication check)
3. CRUSH places PGs on rogue OSD
4. Rogue OSD corrupts replica data
5. Primary OSD replicates to rogue OSD
6. Rogue OSD returns corrupted data to clients

**Impact**: **Data integrity compromise**

**Mitigation** (NOT in plan):

- OSD identity verification (certificate-based or CephX-like)
- Merkle tree checksums for all objects (detect corruption)
- Periodic scrubbing (compare replicas, detect divergence)

### 7.2 S3 Gateway Resource Exhaustion

**Threat Actor**: Internet attacker (if S3 is publicly exposed)

**Attack Steps**:

1. Attacker discovers public S3 endpoint
2. Attacker floods `PUT` requests (no rate limiting)
3. OSDs fill up with garbage objects
4. Cluster runs out of space
5. Legitimate writes fail

**Impact**: **Denial of Service**

**Mitigation** (NOT in plan):

- Per-IP rate limiting (100 req/s)
- Per-account quotas (max objects, max bytes)
- Garbage collection for unsigned objects (delete after timeout)

### 7.3 Recovery Bandwidth Starvation

**Threat Actor**: Malicious client or crash-inducing bug

**Attack Steps**:

1. Attacker crashes OSD (or exploits bug to trigger crash)
2. Cluster starts recovery for all PGs on that OSD
3. Recovery saturates network (no bandwidth limit)
4. Client I/O starves (read/write latency → infinity)
5. Cluster becomes unusable

**Impact**: **Availability loss**

**Mitigation** (NOT in plan):

- Recovery throttling (max 10 concurrent recoveries)
- Bandwidth caps (100 Mbps max for recovery)
- Priority I/O scheduling (client I/O > recovery I/O)

### 7.4 Key Leakage via Unencrypted Metadata

**Threat Actor**: Attacker with physical disk access (stolen disk, decommissioned disk)

**Attack Steps**:

1. Attacker obtains OSD disk (eBay, dumpster diving)
2. Attacker mounts disk, reads iroh-blobs directory
3. Blobs are unencrypted (if plan's "optional encryption" not enabled)
4. Attacker also reads SQLite state machine
5. Metadata reveals object names, sizes, timestamps
6. Attacker reconstructs file structure and contents

**Impact**: **Confidentiality breach**

**Mitigation** (NOT in plan):

- **Mandatory** disk encryption (LUKS)
- Metadata encryption in SQLite
- Secure disk wiping on decommission

---

## 8. Security Implementation Effort Estimate

The plan estimates **11-21 months** for Ceph-like functionality (line 1817-1825). Security is mentioned in **4 lines** (1794-1797).

**Realistic Security Effort**:

| Security Domain | Effort Estimate | Plan Coverage |
|-----------------|-----------------|---------------|
| OSD-to-OSD authentication | 2-3 weeks | ❌ Not mentioned |
| Client-to-OSD authentication (S3) | 3-4 weeks | ❌ "S3Auth" stub only |
| MDS capabilities | 2-3 weeks | ❌ Not mentioned |
| S3 bucket policies + ACLs | 4-6 weeks | ❌ "Per-bucket ACLs" (1 line) |
| Data-at-rest encryption | 6-8 weeks | ❌ "Optional encryption" (1 line) |
| Key management system (KMS) | 4-6 weeks | ❌ Not mentioned |
| S3 gateway rate limiting | 3-4 weeks | ❌ Not mentioned |
| OSD recovery throttling | 2-3 weeks | ❌ Not mentioned |
| MDS DoS protection | 2 weeks | ❌ Not mentioned |
| CRUSH caching (anti-DoS) | 1 week | ❌ Not mentioned |
| Security audit of s3s crate | 2-4 weeks | ❌ Not mentioned |
| **Total Security Work** | **32-54 weeks** | **~0 weeks in plan** |

**Adjusted Timeline**:

- **Plan's Estimate**: 11-21 months
- **Security Addition**: +8-13.5 months
- **Realistic Total**: **19-34.5 months** (1.6-2.9 years)

---

## 9. Recommendations

### 9.1 Immediate Actions

1. **Add Security Phase to Plan**: Create "Phase 0: Security Architecture" (2-3 months)
   - Design authentication for all 6 trust boundaries
   - Define authorization model (S3 policies, MDS capabilities)
   - Choose KMS solution (embedded vs. Vault)
   - Document threat model and attack surfaces

2. **Audit s3s Crate**: Before using s3s:
   - Review source code for CVEs
   - Fuzz S3 request parsing
   - Check for known vulnerabilities in dependencies
   - Consider alternative: Implement minimal S3 subset in-house

3. **Prototype Encryption**: Before Phase 2 (OSDs):
   - Implement encryption wrapper for iroh-blobs
   - Test key rotation procedures
   - Benchmark encryption overhead (target: <10% CPU)

### 9.2 Design Decisions Required

| Decision | Options | Recommendation |
|----------|---------|----------------|
| **OSD Authentication** | CephX-like tickets vs. extend HMAC-SHA256 | CephX-like (industry standard) |
| **S3 Authentication** | SigV4 vs. simple token | SigV4 (AWS compatibility) |
| **Authorization Model** | ACLs vs. policies vs. both | Policies primary, ACLs deprecated |
| **Key Management** | Embedded vs. Vault vs. cloud KMS | Vault (self-hosted, industry standard) |
| **Encryption** | Mandatory vs. optional | Mandatory (security-first) |
| **Metadata Encryption** | Yes vs. no | Yes (defense in depth) |
| **Rate Limiting** | Per-IP vs. per-account vs. both | Both (multi-layer defense) |

### 9.3 Testing Requirements

Security testing **not mentioned** in plan. Add:

1. **Penetration Testing**: Hire security firm to audit implementation
2. **Fuzz Testing**: Use AFL/libFuzzer on:
   - S3 request parsing
   - CRUSH algorithm
   - Encryption layer
   - Authentication protocols

3. **Chaos Engineering**: Simulate:
   - Byzantine OSD (sends corrupted data)
   - DoS attack on S3 gateway
   - Key rotation during I/O

4. **Compliance**: If targeting regulated industries:
   - FIPS 140-2 validated crypto
   - SOC 2 Type II audit
   - GDPR compliance (data deletion guarantees)

### 9.4 Security-First Architecture Principles

1. **Fail Secure**: Default deny, explicit allow (not vice versa)
2. **Defense in Depth**: Multiple security layers (auth + encryption + rate limits)
3. **Least Privilege**: OSDs only access assigned PGs, not all data
4. **Audit Logging**: Log all authentication failures, privilege escalations
5. **Cryptographic Agility**: Support key rotation, algorithm upgrades

---

## 10. Conclusion

The plan to transform Aspen into a Ceph-like storage system **dramatically underestimates security complexity**. Current Aspen has a simple trust model (cluster vs. clients) with production-ready HMAC-SHA256 authentication. The Ceph-like plan introduces:

- **6 new trust boundaries** (vs. 1 currently)
- **4 authentication protocols** (OSD, S3, MDS, Monitor)
- **3 authorization systems** (S3 policies, ACLs, MDS capabilities)
- **2 encryption layers** (network + at-rest)
- **Unknown supply chain risks** (s3s crate audit status unknown)

**Security work is mentioned in 4 lines but requires 8-13.5 months of dedicated effort.**

**Key Risks**:

1. **Data breaches** via unencrypted metadata
2. **DoS attacks** via unbounded S3 requests, recovery bandwidth
3. **Byzantine OSDs** corrupting replica data
4. **Authentication bypass** in S3 gateway or MDS
5. **Supply chain compromise** via unaudited dependencies

**Recommendations**:

1. Add "Phase 0: Security Architecture" (2-3 months)
2. Audit s3s crate before use
3. Make encryption mandatory, not optional
4. Implement rate limiting for all external APIs
5. Conduct penetration testing before production use

**Adjusted Timeline**: 19-34.5 months (not 11-21 months).

---

## References

### Ceph Security Documentation

- [A Detailed Description of the Cephx Authentication Protocol](https://docs.ceph.com/en/reef/dev/cephx_protocol/)
- [CephX Config Reference](https://docs.ceph.com/en/latest/rados/configuration/auth-config-ref/)
- [Chapter 3. Encryption and Key Management (Red Hat)](https://docs.redhat.com/en/documentation/red_hat_ceph_storage/6/html/data_security_and_hardening_guide/assembly-encryption-and-key-management)
- [Chapter 4. Identity and Access Management (Red Hat)](https://docs.redhat.com/en/documentation/red_hat_ceph_storage/8/html/data_security_and_hardening_guide/assembly-identity-and-access-management)

### Ceph CVEs

- [Ceph Security Vulnerabilities (CVE Details)](https://www.cvedetails.com/vulnerability-list/vendor_id-15437/product_id-42195/Ceph-Ceph.html)
- [Past vulnerabilities (Ceph Docs)](https://docs.ceph.com/en/latest/security/cves/)
- [Security Bulletin: IBM Storage Ceph (CVE-2024-34069)](https://www.ibm.com/support/pages/security-bulletin-ibm-storage-ceph-vulnerable-csrf-werkzeug-cve-2024-34069)

### S3 Security

- [Authentication and ACLs (Ceph Docs)](https://docs.ceph.com/en/latest/radosgw/s3/authentication/)
- [Policies and permissions in Amazon S3](https://docs.aws.amazon.com/AmazonS3/latest/userguide/access-policy-language-overview.html)
- [Access control in Amazon S3](https://docs.aws.amazon.com/AmazonS3/latest/userguide/access-management.html)

### Rust Security

- [Rust Foundation's 2025 Technology Report](https://rustfoundation.org/media/rust-foundations-2025-technology-report-showcases-year-of-rust-security-advancements-ecosystem-resilience-strategic-partnerships/)
- [Open sourcing our Rust crate audits (Google)](https://opensource.googleblog.com/2023/05/open-sourcing-our-rust-crate-audits.html?m=1)
- [RustSec Advisory Database](https://github.com/rustsec/advisory-db)

### Related Files in Aspen Codebase

- `/home/brittonr/git/aspen/src/raft/auth.rs` (429 lines): HMAC-SHA256 authentication
- `/home/brittonr/git/aspen/src/raft/constants.rs` (460 lines): Tiger Style resource limits
- `/home/brittonr/git/aspen/src/cluster/gossip_discovery.rs` (lines 267-406): Rate limiting
- `/home/brittonr/git/aspen/.claude/ceph-comparison-analysis-2025-12-20.md`: Original plan
