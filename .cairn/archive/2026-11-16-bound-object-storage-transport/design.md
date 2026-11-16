## Context

`get_bounded` already streams the body with `chunk()` and refuses a chunk that would exceed the remaining bound; the issue was filed against an earlier revision that called `bytes()`. No test exercised a response without `Content-Length`. `validate_config` accepted any numeric `http://` or `https://` endpoint.

## Decisions

### Decision: Pin the streamed bound with a chunked response

**Choice:** A local test server answers with `Transfer-Encoding: chunked` and no length: exactly the bound succeeds, one byte more fails with `TruncatedChunk`, the same class as a declared oversized length.

**Rationale:** The bound must hold without the server's cooperation, and the test proves the client stops reading instead of buffering the remainder.

### Decision: Plain HTTP only on loopback

**Choice:** `validate_config` rejects an `http://` endpoint whose numeric host is not a loopback address. `https://` remains admitted for any numeric endpoint.

**Rationale:** SigV4 protects the credential but not payload confidentiality or integrity. Loopback traffic does not leave the host, which keeps the disposable conformance fixture usable without a separate test profile.

## Risks / Trade-offs

- An operator using plain HTTP to a LAN endpoint must switch to HTTPS; the error names the rule.
- Certificate trust for HTTPS endpoints is unchanged.
