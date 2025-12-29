# Blob CLI Implementation Decision

**Date**: 2025-12-26
**Status**: Implemented

## Context

The aspen-cli binary already had basic blob commands (Add, Get, Has, Ticket, List, Protect, Unprotect), but was missing important operations for complete blob management:

- Deleting blobs from the store
- Downloading blobs from remote peers using tickets
- Getting detailed status information about blobs

## Decision

Implemented three new blob CLI commands with corresponding RPC protocol extensions:

### 1. Delete Command (`blob delete`)

```bash
aspen-cli blob delete <hash> [--force]
```

- Deletes a blob from the store by its BLAKE3 hash
- `--force` flag allows deletion even if the blob is protected
- Note: iroh-blobs uses garbage collection internally, so deletion marks the blob for GC

### 2. Download Command (`blob download`)

```bash
aspen-cli blob download <ticket> [--tag <name>]
```

- Downloads a blob from a remote peer using a BlobTicket
- `--tag` optionally protects the downloaded blob from GC
- Uses iroh-blobs' P2P transfer capabilities

### 3. Status Command (`blob status`)

```bash
aspen-cli blob status <hash>
```

- Returns detailed information about a blob:
  - Whether it exists
  - Size in bytes
  - Completion status (all chunks present)
  - Protection tags

## Implementation Details

### Files Modified

1. **src/client_rpc.rs**:
   - Added `DeleteBlob`, `DownloadBlob`, `GetBlobStatus` request variants
   - Added `DeleteBlobResult`, `DownloadBlobResult`, `GetBlobStatusResult` response variants
   - Added corresponding response structs with proper serialization
   - Updated operation categorization for authorization

2. **src/protocol_handlers/client.rs**:
   - Added handlers for all three new operations
   - Follows existing patterns: blob store check, hash parsing, error sanitization
   - Imports new response types

3. **src/bin/aspen-cli/commands/blob.rs**:
   - Added `Delete`, `Download`, `Status` command variants
   - Added `DeleteArgs`, `DownloadArgs`, `StatusArgs` structs
   - Added `DeleteBlobOutput`, `DownloadBlobOutput`, `BlobStatusOutput` output types
   - Implemented handler functions following existing patterns

### Design Decisions

1. **Delete Implementation**: Since iroh-blobs manages garbage collection internally, the delete operation marks blobs for GC rather than immediate deletion. Protected blobs require `--force` to delete.

2. **Authorization**: New operations are categorized with other blob operations under the `_blob:` prefix:
   - Delete, Download: Write operations
   - Status: Read operation

3. **Error Handling**: Follows Tiger Style and HIGH-4 security pattern:
   - Log full errors internally with `warn!()`
   - Return sanitized error messages to clients
   - Don't expose internal parse errors

4. **Output Formats**: All new commands support both human-readable and JSON output via the `Outputable` trait.

## Alternatives Considered

1. **Direct blob deletion vs GC**: Could have implemented direct file deletion, but this would bypass iroh-blobs' internal consistency mechanisms. Using GC is safer.

2. **Separate download protocols**: Could have used HTTP or a custom protocol, but using existing iroh-blobs ticket mechanism is simpler and leverages built-in P2P capabilities.

## Testing

- Compilation successful with no warnings
- Clippy passes with no errors
- Integration tests run via quick profile

## Usage Examples

```bash
# Add a blob
aspen-cli blob add --data "hello world" --tag mydata
# Output: 2d711642b726b04401627ca9fbac32f5c8530fb1903cc4db02258717921a4881 (11 bytes, added)

# Check status
aspen-cli blob status 2d711642b726b04401627ca9fbac32f5c8530fb1903cc4db02258717921a4881
# Output:
# Hash: 2d711642b726b04401627ca9fbac32f5c8530fb1903cc4db02258717921a4881
# Size: 11 bytes
# Status: complete
# Tags: user:mydata

# Get a ticket for sharing
aspen-cli blob ticket 2d711642b726b04401627ca9fbac32f5c8530fb1903cc4db02258717921a4881
# Output: blob:...

# Download from another node
aspen-cli blob download blob:... --tag downloaded

# Delete a blob
aspen-cli blob delete 2d711642b726b04401627ca9fbac32f5c8530fb1903cc4db02258717921a4881 --force
# Output: deleted
```
