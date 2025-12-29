# DNS CLI Commands Implementation

**Date**: 2025-12-26
**Status**: Implemented

## Summary

Added DNS record and zone management commands to the aspen-cli binary, enabling command-line control of DNS records stored in the distributed cluster.

## Architecture Decision

### RPC Protocol Integration

DNS operations use the existing Client RPC protocol over Iroh QUIC, following the established patterns:

1. **Request types added to `ClientRpcRequest`**:
   - `DnsSetRecord` - Create/update DNS records
   - `DnsGetRecord` - Get single record by domain/type
   - `DnsGetRecords` - Get all records for a domain
   - `DnsDeleteRecord` - Delete a specific record
   - `DnsResolve` - Resolution with wildcard fallback
   - `DnsScanRecords` - Prefix-based record scanning
   - `DnsSetZone` - Create/update zones
   - `DnsGetZone` - Get zone by name
   - `DnsListZones` - List all zones
   - `DnsDeleteZone` - Delete zone with optional record cleanup

2. **Response types added to `ClientRpcResponse`**:
   - `DnsRecordResultResponse` - Single record operation
   - `DnsRecordsResultResponse` - Multiple records
   - `DnsZoneResultResponse` - Zone operation
   - `DnsZonesResultResponse` - Zone list
   - `DnsDeleteRecordResultResponse` / `DnsDeleteZoneResultResponse`

### CLI Command Structure

Commands follow the existing CLI patterns:

```
aspen-cli dns set <domain> <type> [value] [--data <json>] [--ttl <seconds>]
aspen-cli dns get <domain> <type>
aspen-cli dns get-all <domain>
aspen-cli dns delete <domain> <type>
aspen-cli dns resolve <domain> <type>
aspen-cli dns scan [prefix] [--limit <n>]
aspen-cli dns zone set <name> [--default-ttl <seconds>] [--description <text>] [--disabled]
aspen-cli dns zone get <name>
aspen-cli dns zone list
aspen-cli dns zone delete <name> [--delete-records]
```

### Key Implementation Details

1. **Record Data Serialization**: Record data is passed as JSON strings over the RPC protocol, allowing flexible type-specific data structures.

2. **Authorization Integration**: DNS operations are integrated into the capability token system:
   - Read operations use `dns:{domain}` key prefix
   - Write operations use `dns:{domain}` for records, `dns:_zone:{name}` for zones

3. **AspenDnsStore Compatibility**: Updated to use `?Sized` bound to support `Arc<dyn KeyValueStore>` in the protocol handler.

4. **Output Formatting**: Zone file-style output for human-readable format:

   ```
   api.example.com    300 IN A     192.168.1.1
   ```

## Files Modified

- `src/client_rpc.rs` - Added DNS request/response types and auth operations
- `src/dns/store.rs` - Added `?Sized` bound to AspenDnsStore
- `src/protocol_handlers/client.rs` - Added DNS request handlers
- `src/bin/aspen-cli/commands/dns.rs` - New DNS command module
- `src/bin/aspen-cli/commands/mod.rs` - Added dns module export
- `src/bin/aspen-cli/cli.rs` - Wired up DnsCommand
- `src/bin/aspen-cli/output.rs` - Added DNS output types

## Trade-offs

1. **JSON Data Format**: Using JSON for record data provides flexibility but adds serialization overhead. Alternative was to add specific RPC variants for each record type, but this would significantly increase code complexity.

2. **Record Type Validation**: The record type is both specified as a string parameter AND embedded in the JSON data. We validate they match to prevent inconsistencies.

3. **Zone Operations**: Zones are currently metadata-only (no enforced record containment). Records exist independently and zones serve as organizational/configuration units.

## Future Considerations

1. Add `--json` output examples to help documentation
2. Consider batch import/export commands for zone file format
3. Add DNS ticket generation command for client distribution
