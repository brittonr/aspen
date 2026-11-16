## Why

Issue #50 found two gaps in the optional S3 client (`content-store-object-storage`). A response without `Content-Length` could be buffered before the object bound was checked, and plain `http://` endpoints were admitted for any address, leaving payload confidentiality and integrity unprotected on a network path.

## What Changes

- Confirm and pin the streamed GET bound: a chunked response without `Content-Length` stops at the bound and is classified like any oversized object.
- Admit plain `http://` only for loopback endpoints; non-loopback endpoints must use `https://`. The disposable local SeaweedFS fixture at `http://127.0.0.1:28433` keeps working.

## Impact

- **Files**: `src/content_store_adapter/live/s3.rs`, the content-store adapter documentation and the chunk-store specification.
- **Testing**: endpoint admission for loopback/non-loopback IPv4 and IPv6 over HTTP and HTTPS; a chunked GET at and above the bound against a local server.
- **Owner / consumer**: the object-storage content-store profile; hosted conformance uses the loopback fixture.
