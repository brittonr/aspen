## Why

The service-host bundle codec treats every field whose name ends in `_ref` or `_refs` as a
content reference. Fabric port descriptors name their schemas by contract label
(`molten.fabric.transport.command.v1`), and the collaboration commit effect names its
input/output schemas the same way, so a bundle carrying a real admitted manifest could be
encoded but never decoded. The collaboration native effect rail hit this on its first real
startup: `content ref must start with blake3:, got molten.fabric.transport.command.v1`.

## What Changes

- Admit schema references as contract labels in the service-host codec instead of content
  references, keeping every real content reference checked. r[molten.system_extension.hosted_codec]

## Impact

- **Files**: `src/node/servicehost/codec/wire.rs` and its validation tests.
- **Testing**: the codec unit tests (including a new schema-label regression) and the node
  service-host library tests. No admission, authority or production claim changes.
