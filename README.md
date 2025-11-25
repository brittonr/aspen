# Blixard (mvm-ci)

A distributed job orchestrator with heterogeneous worker support (WASM/Firecracker) and P2P networking.

## Architecture

- **Distributed Queue**: Raft-replicated SQLite (Hiqlite) for consensus
- **P2P Networking**: Iroh with HTTP/3 over QUIC for inter-node communication
- **Worker Types**: Supports both WASM (Flawless) and Firecracker MicroVMs
- **Design Pattern**: Clean Architecture/DDD with CQRS and repository pattern

## MVP Setup

### Quick Start

1. **Copy and configure environment variables:**
   ```bash
   cp .env.example .env
   # Edit .env and set your secrets (use openssl rand -hex 32 for generation)
   ```

2. **Generate hiqlite.toml from template:**
   ```bash
   envsubst < hiqlite.toml.template > hiqlite.toml
   ```

3. **Run the server:**
   ```bash
   nix develop -c cargo run
   ```

### API Authentication

All API endpoints now require authentication via the `X-API-Key` header:

```bash
# Example: Submit a job
curl -X POST http://localhost:3020/api/queue/publish \
  -H "Content-Type: application/json" \
  -H "X-API-Key: YOUR_API_KEY_FROM_ENV" \
  -d '{
    "id": "test-job-1",
    "payload": "{\"task\": \"example\"}",
    "priority": 1
  }'
```

### Protected Endpoints

The following API routes require authentication:
- `/api/queue/*` - Work queue operations
- `/api/workers/*` - Worker management
- `/api/iroh/*` - P2P operations

### Dashboard

The dashboard remains publicly accessible at:
- `http://localhost:3020/dashboard`

## Security Improvements (MVP)

✅ API key authentication middleware
✅ Secrets moved to environment variables
✅ Fixed critical production unwraps
✅ Secure configuration templates

## Development

### Building

```bash
nix develop -c cargo build
```

### Testing

```bash
nix develop -c cargo test
```

## Troubleshooting

- If you get authentication errors, verify your API key matches the one in .env
- Ensure all environment variables are set before running
- Check that hiqlite.toml was generated correctly from the template

## Future Enhancements

While not critical for MVP, these improvements are planned:
- Lazy cache refresh with TTL (currently O(n) on every operation)
- Connection pooling for iroh client
- Structured error responses for better API UX
- Enhanced health check endpoint with detailed metrics
- JWT/OAuth2 authentication to replace simple API keys
- Prometheus metrics and Grafana dashboards
- Multi-region deployment support