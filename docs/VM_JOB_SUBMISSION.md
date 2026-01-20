# VM Job Submission with Blob Storage

## Overview

Aspen now uses iroh-blobs for efficient VM binary storage, eliminating JSON/base64 overhead and enabling P2P distribution.

## Architecture Changes

### Before (Base64 in JSON)

```
Binary (52KB) → Base64 (70KB) → JSON → Postcard → Network
```

### After (Direct Blob Storage)

```
Binary (52KB) → Blob Store → BLAKE3 Hash → Network (just 32-byte hash!)
```

## Benefits

- **33% bandwidth reduction** - No base64 encoding overhead
- **P2P distribution** - Nodes fetch binaries from each other via iroh-blobs protocol
- **Automatic deduplication** - Same binary = same hash = stored once
- **Content-addressed caching** - BLAKE3 hashes ensure integrity
- **Efficient streaming** - Large binaries can be streamed, not loaded in memory

## New Submission Flow

### 1. Upload Binary to Blob Store

```rust
use aspen_blob::BlobStore;

// Get blob store from node
let blob_store = node.blob_store().unwrap();

// Upload binary
let binary = std::fs::read("path/to/binary")?;
let result = blob_store.add_bytes(&binary).await?;

let hash = result.blob_ref.hash.to_string();
let size = result.blob_ref.size;
```

### 2. Create Job with Blob Reference

```rust
use aspen_jobs::JobSpec;

// Create job spec with blob hash (NOT the binary itself!)
let job_spec = JobSpec::with_blob_binary(hash, size, "elf")
    .timeout(Duration::from_secs(10))
    .tag("my-vm-job");
```

### 3. Submit via RPC

```rust
// The payload now contains just the hash reference
ClientRpcRequest::JobSubmit {
    job_type: "vm_execute".to_string(),
    payload: serde_json::json!({
        "type": "BlobBinary",
        "hash": hash,
        "size": size,
        "format": "elf"
    }),
    // ... other fields
}
```

## Worker Execution

The HyperlightWorker automatically:

1. Receives job with blob hash
2. Retrieves binary from local blob store
3. If not found locally, fetches from peer nodes via iroh-blobs protocol
4. Executes in Hyperlight VM
5. Caches the blob hash for future use

## Example: Complete Flow

```rust
// 1. Start node with blob storage
let node = NodeBuilder::new()
    .enable_blob_storage()
    .build()
    .await?;

// 2. Upload binary
let blob_store = node.blob_store().unwrap();
let binary = std::fs::read("echo-worker")?;
let result = blob_store.add_bytes(&binary).await?;

// 3. Submit job
let job_spec = JobSpec::with_blob_binary(
    result.blob_ref.hash.to_string(),
    result.blob_ref.size,
    "elf"
);

// 4. VM executes with binary fetched from blob store
```

## Usage

All VM binaries must be uploaded to blob storage first, then referenced by hash:

```rust
// Store binary in blobs first
let result = blob_store.add_bytes(&binary_data).await?;

// Create job spec with blob reference
let job_spec = JobSpec::with_blob_binary(
    result.blob_ref.hash.to_string(),
    result.blob_ref.size,
    "elf"
);
```

The `JobPayload` enum supports three variants:

- `BlobBinary { hash, size, format }` - VM binary stored in blob store
- `NixExpression { flake_url, attribute }` - Nix flake reference
- `NixDerivation { content }` - Inline Nix derivation

## P2P Binary Distribution

When a node needs a binary:

1. **Check local store** - O(1) lookup by hash
2. **Query cluster peers** - "Who has hash X?"
3. **Fetch via iroh-blobs** - Direct QUIC transfer
4. **Cache locally** - Future jobs use local copy

## Performance Impact

- **Upload once**: Binary uploaded once, referenced many times
- **Network efficiency**: 52KB binary → 32-byte hash in job payload
- **Deduplication**: 100 jobs with same binary = 1 blob stored
- **P2P speedup**: Parallel downloads from multiple peers

## Security Considerations

- **Integrity**: BLAKE3 hash ensures binary hasn't been tampered
- **No trust needed**: Content-addressing means hash IS the identity
- **Efficient validation**: Hash checked on retrieval

## CLI Usage (Future)

```bash
# Upload binary to cluster
aspen blob add echo-worker
# Returns: Hash: 3b99a8d846f3d553...

# Submit VM job with hash
aspen job submit \
  --type vm_execute \
  --binary-hash 3b99a8d846f3d553... \
  --input "Hello, World!"
```

## Debugging

Check if binary exists in blob store:

```rust
let exists = blob_store.get_bytes(&hash).await?.is_some();
```

List protected blobs (won't be garbage collected):

```rust
let tags = blob_store.list_protected().await?;
```

## Best Practices

1. **Upload binaries once** - Store hash for reuse
2. **Use protection tags** - Prevent GC of important binaries
3. **Validate size** - Include size in BlobBinary for validation
4. **Format hints** - Specify "elf", "wasm", etc. for clarity
5. **Error handling** - Handle "blob not found" gracefully
