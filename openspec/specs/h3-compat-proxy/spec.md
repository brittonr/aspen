## h3-compat-proxy

Local TCP-to-iroh-h3 proxy for browser and standard HTTP client access.

### Requirements

1. The proxy MUST listen on a configurable TCP address (default `127.0.0.1:8080`) and accept HTTP/1.1 requests.
2. The proxy MUST forward each HTTP request as an HTTP/3 request to the specified iroh endpoint ID over QUIC, using the configured ALPN for protocol negotiation.
3. The proxy MUST preserve the request method, path, query string, headers, and body when forwarding.
4. The proxy MUST stream the h3 response body back to the TCP client without buffering the entire response in memory.
5. The proxy MUST return HTTP 502 Bad Gateway when the iroh endpoint is unreachable or the h3 request fails.
6. The proxy MUST reconnect to the iroh endpoint automatically if the QUIC connection drops, with exponential backoff (1s, 2s, 4s, max 30s).
7. The proxy MUST support concurrent requests — multiple TCP connections handled simultaneously via multiplexed h3 streams on a single QUIC connection.
8. The proxy MUST expose a library API (`H3Proxy::new`, `H3Proxy::run`) for embedding in other binaries without spawning a subprocess.
9. The proxy binary MUST accept `--endpoint-id`, `--alpn`, and `--port` as CLI arguments.
10. The proxy MUST bind to localhost by default. Binding to `0.0.0.0` requires explicit `--bind 0.0.0.0` flag.

### Acceptance Criteria

- `aspen-h3-proxy --endpoint-id <id> --alpn "aspen/forge-web/1" --port 8080` starts and proxies `curl http://localhost:8080/` to the forge web frontend
- `aspen-h3-proxy --endpoint-id <id> --alpn "iroh+h3" --port 8380` proxies nix cache requests: `nix path-info --store http://localhost:8380 <path>` returns valid narinfo
- Response headers (content-type, content-length) pass through correctly
- Large responses (>1MB NAR files) stream without OOM
- Proxy recovers after target endpoint restart within 30 seconds
- Library API compiles and links when used as a dependency from another crate
