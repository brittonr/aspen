# Getting Started with VM Cluster Testing

## Current Status

The full VM test requires:
1. ✅ NixOS test framework (available in your Nix setup)
2. ✅ mvm-ci Rust package (builds from your flake)
3. ⏳ Flawless packaged for Nix (needs to be added)

## Quick Start: Incremental Approach

### Step 1: Verify NixOS Test Framework Works

Test that you can run NixOS VM tests:

```bash
# Simple test to verify QEMU works
cat > /tmp/test-vm.nix <<'EOF'
import <nixpkgs/nixos/tests/make-test-python.nix> ({ pkgs, ... }: {
  name = "simple-test";
  nodes = {
    machine = { config, pkgs, ... }: {
      environment.systemPackages = [ pkgs.hello ];
    };
  };
  testScript = ''
    start_all()
    machine.wait_for_unit("multi-user.target")
    output = machine.succeed("hello")
    print(f"Got: {output}")
    assert "Hello" in output
  '';
})
EOF

# Run it
nix-build /tmp/test-vm.nix
```

**Expected output:**
```
machine: starting vm
machine: waiting for unit multi-user.target
Got: Hello, world!
test script finished in 12.34s
```

### Step 2: Add Flawless to Your Nix Flake

Update your `flake.nix` to package flawless:

```nix
{
  outputs = { self, nixpkgs, ... }: {
    packages.x86_64-linux = {
      # ... existing packages ...

      flawless = pkgs.rustPlatform.buildRustPackage {
        pname = "flawless";
        version = "1.0.0-beta.3";

        src = pkgs.fetchFromGitHub {
          owner = "sebadob";
          repo = "flawless";
          rev = "v1.0.0-beta.3";
          sha256 = "...";  # Get with nix-prefetch-github
        };

        cargoLock = {
          lockFile = ./Cargo.lock;
        };
      };
    };
  };
}
```

### Step 3: Create Minimal 2-Node Test

Start with a simpler test:

```nix
# tests/vm-minimal-test.nix
{ pkgs ? import <nixpkgs> {} }:

pkgs.nixosTest {
  name = "mvm-ci-minimal";

  nodes = {
    node1 = { config, pkgs, ... }: {
      networking.firewall.enable = false;

      systemd.services.test-service = {
        wantedBy = [ "multi-user.target" ];
        script = ''
          ${pkgs.python3}/bin/python3 -m http.server 3020
        '';
      };
    };

    node2 = { config, pkgs, ... }: {
      networking.firewall.enable = false;
    };
  };

  testScript = ''
    start_all()
    node1.wait_for_open_port(3020)

    # Test cross-VM communication
    output = node2.succeed("curl http://node1:3020/")
    print(f"Node2 can reach Node1: {output[:100]}")

    # Test network partition
    node1.succeed("iptables -A INPUT -s node2 -j DROP")
    node2.fail("curl --max-time 2 http://node1:3020/")
    print("Network partition working")

    # Heal partition
    node1.succeed("iptables -F")
    node2.succeed("curl http://node1:3020/")
    print("Partition healed")
  '';
}
```

Run it:
```bash
nix-build tests/vm-minimal-test.nix
```

### Step 4: Add mvm-ci Without Flawless

Test mvm-ci in VMs without flawless first:

```nix
# tests/vm-mvm-ci-only.nix
{ pkgs ? import <nixpkgs> {} }:

let
  mvm-ci = pkgs.callPackage ../. {};  # Your existing package
in
pkgs.nixosTest {
  name = "mvm-ci-basic";

  nodes.machine = { config, pkgs, ... }: {
    environment.systemPackages = [ mvm-ci ];
  };

  testScript = ''
    start_all()
    machine.wait_for_unit("multi-user.target")

    # Verify binary exists and runs
    output = machine.succeed("mvm-ci --version")
    print(f"mvm-ci version: {output}")
  '';
}
```

### Step 5: Test Hiqlite Cluster in VMs

Once that works, add hiqlite clustering:

```nix
nodes = {
  node1 = mkNode 1;
  node2 = mkNode 2;
};

# Each node gets a hiqlite config pointing to both nodes
```

## Troubleshooting

### "flawless: command not found"

Flawless isn't packaged yet. Options:
1. Package it yourself (Step 2)
2. Download pre-built binary:
   ```bash
   curl -L https://github.com/sebadob/flawless/releases/download/v1.0.0-beta.3/flawless -o flawless
   ```
3. Use `installPhase` to copy binary into Nix store

### VMs Too Slow

Reduce to 2 nodes:
```nix
nodes = {
  node1 = ...;
  node2 = ...;
};
```

Give more RAM:
```nix
virtualisation.memorySize = 2048;  # 2GB per VM
```

### Can't Connect to VMs

Check networking:
```python
>>> node1.succeed("ip addr")
>>> node1.succeed("ping -c 1 node2")
```

## Alternative: Docker Compose

If QEMU is too heavy, use Docker Compose for testing:

```yaml
# docker-compose.yml
version: '3.8'

services:
  node1:
    build: .
    ports:
      - "3020:3020"
    environment:
      - NODE_ID=1
      - HTTP_PORT=3020
    networks:
      cluster:
        ipv4_address: 172.20.0.2

  node2:
    build: .
    ports:
      - "3021:3020"
    environment:
      - NODE_ID=2
      - HTTP_PORT=3020
    networks:
      cluster:
        ipv4_address: 172.20.0.3

networks:
  cluster:
    driver: bridge
    ipam:
      config:
        - subnet: 172.20.0.0/16
```

Test with:
```bash
docker-compose up -d
docker-compose exec node1 curl http://node2:3020/queue/list
```

## Next Steps

1. ✅ Verify QEMU works (Step 1)
2. ⏳ Package flawless (Step 2)
3. ⏳ Test minimal 2-node cluster (Step 3)
4. ⏳ Add mvm-ci to VMs (Step 4)
5. ⏳ Add hiqlite clustering (Step 5)
6. ⏳ Run full test suite (`vm-cluster-test.nix`)

## Learning Resources

- [NixOS Test Documentation](https://nixos.org/manual/nixos/stable/#sec-nixos-tests)
- [Writing NixOS Tests](https://nixos.wiki/wiki/NixOS_Testing_library)
- [Example Tests in Nixpkgs](https://github.com/NixOS/nixpkgs/tree/master/nixos/tests)
