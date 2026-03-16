## ADDED Requirements

### Requirement: Manifest parsing

The `aspen-sops-install-secrets` binary SHALL parse the sops-nix manifest JSON from a file path argument. The manifest schema matches the Go `sops-install-secrets` format: secrets array, templates array, placeholderBySecretName map, secretsMountPoint, symlinkPath, keepGenerations, key source fields (ageKeyFile, ageSshKeyPaths, sshKeyPaths, gnupgHome), useTmpfs, userMode, and logging config.

#### Scenario: Valid manifest parses successfully

- **WHEN** `aspen-sops-install-secrets manifest.json` is invoked with a valid manifest
- **THEN** the binary parses all fields and proceeds to secret installation

#### Scenario: Invalid manifest produces clear error

- **WHEN** the manifest JSON is malformed or missing required fields
- **THEN** the binary exits with a non-zero code and prints a descriptive error to stderr

#### Scenario: Unknown fields are tolerated

- **WHEN** the manifest contains fields not recognized by the Rust structs
- **THEN** parsing succeeds (forward compatibility with future sops-nix versions)

### Requirement: Check mode validation

The binary SHALL support `-check-mode` flag with values `manifest`, `sopsfile`, and `off` (default). In `manifest` mode, it validates JSON structure only. In `sopsfile` mode, it also verifies SOPS files exist and contain expected keys. Both modes exit without installing secrets.

#### Scenario: Manifest check mode

- **WHEN** invoked with `-check-mode=manifest`
- **THEN** the binary validates the manifest structure and exits 0 on success

#### Scenario: Sopsfile check mode validates key paths

- **WHEN** invoked with `-check-mode=sopsfile` and a secret references key `database/password` in a YAML file
- **THEN** the binary verifies the SOPS file exists and contains the key path, exiting non-zero if missing

### Requirement: Secret decryption with Aspen Transit

The binary SHALL decrypt SOPS files using `aspen_secrets::sops::decrypt` with Aspen Transit as the primary backend. The cluster ticket SHALL be read from the `ASPEN_CLUSTER_TICKET` environment variable.

#### Scenario: Transit decrypt with cluster ticket

- **WHEN** `ASPEN_CLUSTER_TICKET` is set and the SOPS file contains `aspen_transit` metadata
- **THEN** the data key is decrypted via Transit and the secret value is extracted

#### Scenario: Missing cluster ticket falls back to age

- **WHEN** `ASPEN_CLUSTER_TICKET` is not set but `ageKeyFile` is configured in the manifest
- **THEN** decryption falls back to age using the configured key file

### Requirement: Age key file support

The binary SHALL support age key files specified in the manifest's `ageKeyFile` field. It SHALL set the `SOPS_AGE_KEY_FILE` environment variable pointing to a temporary key file containing all imported age keys.

#### Scenario: Age key file configured

- **WHEN** `ageKeyFile` is `/var/lib/sops-nix/key.txt`
- **THEN** the age key is loaded and used for SOPS decryption

### Requirement: SSH-to-age key conversion

The binary SHALL convert SSH ed25519 private keys listed in the manifest's `ageSshKeyPaths` to age identities. Converted keys are appended to the temporary age key file alongside any explicit age keys.

#### Scenario: SSH ed25519 key converted to age

- **WHEN** `ageSshKeyPaths` contains `/etc/ssh/ssh_host_ed25519_key`
- **THEN** the ed25519 key material is converted to an age identity and used for decryption

#### Scenario: Non-ed25519 SSH keys are skipped

- **WHEN** `ageSshKeyPaths` contains an RSA key
- **THEN** the key is skipped with a warning, and decryption continues with remaining keys

### Requirement: Key extraction from decrypted documents

After decrypting a SOPS file, the binary SHALL extract the value at the key path specified in each secret's `key` field. An empty key means the entire decrypted file is used as the secret value.

#### Scenario: Nested key extraction

- **WHEN** a secret has `key: "database/password"` and format `yaml`
- **THEN** the binary traverses the YAML structure and extracts the string value at `database.password`

#### Scenario: Empty key uses whole file

- **WHEN** a secret has `key: ""` and format `json`
- **THEN** the entire decrypted JSON document is used as the secret value

#### Scenario: Binary format uses raw bytes

- **WHEN** a secret has format `binary`
- **THEN** the raw decrypted bytes are written without parsing

### Requirement: Secrets filesystem mount

The binary SHALL mount a ramfs (or tmpfs when `useTmpfs` is true) at the `secretsMountPoint` path. The mount SHALL be created if not already present.

#### Scenario: Ramfs mount on first activation

- **WHEN** `secretsMountPoint` is `/run/secrets.d` and no mount exists
- **THEN** a ramfs is mounted at `/run/secrets.d` with mode 0751

#### Scenario: Tmpfs mount when configured

- **WHEN** `useTmpfs` is true
- **THEN** a tmpfs is mounted instead of ramfs

#### Scenario: Existing mount is reused

- **WHEN** a ramfs is already mounted at `secretsMountPoint`
- **THEN** the mount is kept and a new generation directory is created within it

### Requirement: Secret file installation

The binary SHALL write each decrypted secret to `<secretsMountPoint>/<generation>/<name>` with the mode, owner, and group specified in the manifest.

#### Scenario: Secret written with correct permissions

- **WHEN** a secret has mode `0400`, owner `root`, group `root`
- **THEN** the file is created with mode 0400 owned by root:root

#### Scenario: Owner and group resolved by name

- **WHEN** a secret has `owner: "nginx"` and `group: "nginx"`
- **THEN** the UID and GID are looked up from `/etc/passwd` and `/etc/group`

#### Scenario: UID/GID used when names are null

- **WHEN** a secret has `owner: null`, `uid: 1000`, `group: null`, `gid: 1000`
- **THEN** the file is created with uid 1000 and gid 1000

### Requirement: Symlink management

The binary SHALL atomically update the `symlinkPath` (default `/run/secrets`) to point to the current generation directory. Per-secret symlinks SHALL be created at each secret's `path` field.

#### Scenario: Atomic symlink update

- **WHEN** generation 5 is installed
- **THEN** `/run/secrets` is atomically swapped to point to `/run/secrets.d/5`

#### Scenario: Per-secret symlinks

- **WHEN** a secret has `path: "/run/secrets/db-password"`
- **THEN** a symlink is created at `/run/secrets/db-password` pointing to the generation file

### Requirement: Generation pruning

The binary SHALL remove old generation directories, keeping at most `keepGenerations` generations. Setting `keepGenerations` to 0 disables pruning.

#### Scenario: Prune old generations

- **WHEN** `keepGenerations` is 1 and generation 5 is current
- **THEN** generations 1 through 3 are removed, generation 4 is kept (1 previous + current)

#### Scenario: Pruning disabled

- **WHEN** `keepGenerations` is 0
- **THEN** no generations are removed

### Requirement: Template rendering

The binary SHALL render templates by replacing placeholder strings with decrypted secret values. Placeholders are defined in `placeholderBySecretName`. Rendered templates are written to `<generation>/rendered/<name>`.

#### Scenario: Template with placeholder substitution

- **WHEN** a template contains `<<SOPS_PLACEHOLDER_db-password>>` and the secret `db-password` decrypts to `hunter2`
- **THEN** the rendered output contains `hunter2` at that position

#### Scenario: Template from file

- **WHEN** a template has `file: "/path/to/template.conf"` and empty `content`
- **THEN** the file is read and used as the template source

### Requirement: Service restart tracking

The binary SHALL track which secrets/templates changed between generations and write affected unit names to `/run/nixos/activation-restart-list` and `/run/nixos/activation-reload-list`.

#### Scenario: Changed secret triggers restart

- **WHEN** secret `db-password` changes between generations and has `restartUnits: ["myapp.service"]`
- **THEN** `myapp.service` is appended to `/run/nixos/activation-restart-list`

#### Scenario: Dry activation writes to dry prefix

- **WHEN** `NIXOS_ACTION=dry-activate`
- **THEN** restart/reload lists are written under `/run/nixos/dry-activation-*` instead

### Requirement: Format support

The binary SHALL support `yaml`, `json`, and `binary` formats for secret decryption. Unsupported formats (`dotenv`, `ini`) SHALL produce a clear error.

#### Scenario: YAML format decryption

- **WHEN** a secret has format `yaml`
- **THEN** the SOPS file is decrypted and parsed as YAML for key extraction

#### Scenario: Unsupported format error

- **WHEN** a secret has format `dotenv`
- **THEN** the binary exits with an error: "format 'dotenv' is not supported; use yaml or json"

### Requirement: Logging configuration

The binary SHALL respect the manifest's `logging` config: `keyImport` controls whether key import messages are printed, `secretChanges` controls whether secret modification summaries are printed.

#### Scenario: Key import logging enabled

- **WHEN** `logging.keyImport` is true and an SSH key is converted
- **THEN** a message is printed: "Imported /path/to/key as age key with fingerprint ..."

#### Scenario: Secret changes logging disabled

- **WHEN** `logging.secretChanges` is false
- **THEN** no "adding/modifying/removing N secrets" messages are printed
