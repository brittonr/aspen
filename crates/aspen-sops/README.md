# aspen-sops

SOPS backend using Aspen Transit for key management.

Encrypt/decrypt SOPS files using Aspen's distributed Transit engine as the key
management backend. Compatible with the standard SOPS file format.

## Usage

### Encrypt

```bash
aspen-sops encrypt \
    --cluster-ticket aspen1q... \
    --transit-key sops-data-key \
    secrets.toml > secrets.sops.toml
```

### Decrypt

```bash
aspen-sops decrypt \
    --cluster-ticket aspen1q... \
    secrets.sops.toml
```

### Edit

```bash
aspen-sops edit \
    --cluster-ticket aspen1q... \
    secrets.sops.toml
```

### Rotate

Re-wrap the data key with the latest Transit key version (after Transit key
rotation):

```bash
aspen-sops rotate \
    --cluster-ticket aspen1q... \
    --in-place \
    secrets.sops.toml
```

### Update Keys

Add or remove key groups:

```bash
# Add age recipient for offline fallback
aspen-sops update-keys \
    --cluster-ticket aspen1q... \
    --add-age age1... \
    --in-place \
    secrets.sops.toml
```

## Multi-Key-Group

Files can have both Aspen Transit and age key groups. This allows:

- **Transit**: Primary decryption via Aspen cluster
- **Age**: Offline fallback when cluster is unavailable

```bash
aspen-sops encrypt \
    --cluster-ticket aspen1q... \
    --age age1ql3z7hjy54pw3hyww5ayyfg7zqgvc7w3j2elw8zmrj2kg5sfn9aqmcac8p \
    secrets.toml > secrets.sops.toml
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `ASPEN_CLUSTER_TICKET` | Default cluster ticket (avoids `--cluster-ticket` flag) |

## CI Integration

```bash
# In CI pipeline — decrypt secrets using the cluster
export ASPEN_CLUSTER_TICKET="aspen1q..."
API_KEY=$(aspen-sops decrypt --extract secrets.api_key secrets.sops.toml)
```

## Architecture

The library code lives in `aspen-secrets::sops`. This crate is a CLI wrapper
that provides the `aspen-sops` binary. See `docs/sops.md` for the full design.
