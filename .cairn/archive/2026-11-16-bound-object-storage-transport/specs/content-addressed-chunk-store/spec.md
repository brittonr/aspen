# Content-Addressed Chunk Store Delta

## MODIFIED Requirements

### Requirement: Content operations are resource bounded
r[molten.content_store_adapter.streaming_bounds] Molten MUST enforce admitted bounds for total bytes, chunk count, chunk size, range size, concurrent operations, queued bytes, memory, deadline, retry, and cancellation on content adapter operations. Adapters MUST NOT require unbounded buffering or hidden queues.

The client MUST admit numeric endpoints, disable redirects/proxies, freeze credentials and endpoint for its lifetime, and fail excess outstanding requests before dispatch. It MUST bound connect/request time and body buffering, including responses without a declared length. It MUST admit plain HTTP only for loopback endpoints.

#### Scenario: Invalid resolution policy
- GIVEN a hostname endpoint
- WHEN the client is constructed
- THEN admission fails without DNS or network I/O.

#### Scenario: Plain HTTP to a non-loopback endpoint
- GIVEN a numeric `http://` endpoint that is not a loopback address
- WHEN the client is constructed
- THEN admission fails, while the same endpoint over `https://` and a loopback `http://` endpoint are admitted.

#### Scenario: Bounded stream completes
- GIVEN content and operation sizes remain within the admitted profile
- WHEN a stream is read and verified
- THEN progress and terminal events MUST remain within declared resource bounds.

#### Scenario: Oversized object is denied
- GIVEN a manifest or remote response exceeds the admitted byte or chunk bound
- WHEN adapter preflight or streaming validation detects it
- THEN the operation MUST deny or cancel before excess bytes are exposed or indexed.

#### Scenario: Response without a declared length exceeds the bound
- GIVEN a remote GET response streams more bytes than the bound without a `Content-Length`
- WHEN the client reads the body
- THEN it MUST stop at the bound and classify the response as oversized rather than buffering the remainder.
