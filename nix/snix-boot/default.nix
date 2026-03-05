# SNIX Boot Infrastructure
#
# Builds all components needed to boot a microVM with /nix/store served
# by snix's VirtioFS daemon:
#
#   1. snix-store binary (Rust, with virtiofs feature)
#   2. snix-init (Go, minimal VM init)
#   3. Linux kernel with virtiofs support
#   4. u-root initrd containing snix-init
#   5. runVM script tying it all together
#
# Based on https://git.snix.dev/snix/snix/src/branch/canon/snix/boot
{
  lib,
  pkgs,
  snix-src,
  crane,
  rust-overlay ? null,
  aspen-snix-bridge ? null,
}: let
  # ── snix-store binary ─────────────────────────────────────────────
  # Build the snix-store CLI from the snix workspace source.
  # This binary provides `snix-store daemon`, `snix-store virtiofs`,
  # `snix-store import`, and `snix-store copy` subcommands.
  craneLib = crane.mkLib pkgs;

  # The snix workspace lives at snix/ inside the repo.
  # We use the full source since it's a flake input (store path).
  snixWorkspaceSrc = snix-src;

  # Prepare snix source: copy into writable tree so crane can find Cargo.lock
  snixSrc = pkgs.runCommand "snix-workspace-src" {} ''
    cp -r ${snixWorkspaceSrc}/snix $out
    chmod -R u+w $out
  '';

  commonSnixArgs = {
    pname = "snix-store";
    version = "0.1.0";
    src = snixSrc;

    # Build only the snix-store binary with virtiofs support
    cargoExtraArgs = "-p snix-cli-store --features virtiofs";

    nativeBuildInputs = with pkgs; [
      protobuf # protoc for snix-castore build.rs
      pkg-config
    ];

    buildInputs = with pkgs;
      [
        openssl
      ]
      ++ lib.optionals stdenv.isDarwin [
        darwin.apple_sdk.frameworks.Security
      ];

    # snix-castore build.rs needs PROTO_ROOT to find proto files
    PROTO_ROOT = snix-src;

    # Don't run tests during build
    doCheck = false;
  };

  snixStoreCargoArtifacts = craneLib.buildDepsOnly commonSnixArgs;

  snix-store = craneLib.buildPackage (commonSnixArgs
    // {
      cargoArtifacts = snixStoreCargoArtifacts;

      # Install as 'snix-store'
      postInstall = ''
        # The binary is already named snix-store from the [[bin]] in Cargo.toml
        test -f $out/bin/snix-store || {
          echo "ERROR: snix-store binary not found"
          ls -la $out/bin/
          exit 1
        }
      '';
    });

  # ── snix-init (Go) ───────────────────────────────────────────────
  # Minimal init binary for the VM. Mounts virtiofs, handles cmdline.
  snix-init = pkgs.buildGoModule rec {
    name = "snix-init";
    src = lib.fileset.toSource {
      root = ./.;
      fileset = ./snix-init.go;
    };
    vendorHash = null;
    postPatch = "go mod init ${name}";
  };

  # ── Kernel ────────────────────────────────────────────────────────
  # Minimal kernel with virtiofs support baked in.
  kernel = pkgs.buildLinux (
    {}
    // {
      inherit (pkgs.linuxPackages_latest.kernel) src version modDirVersion;
      autoModules = false;
      kernelPreferBuiltin = true;
      ignoreConfigErrors = true;
      kernelPatches = [];
      structuredExtraConfig = with pkgs.lib.kernel; {
        FUSE_FS = option yes;
        DAX_DRIVER = option yes;
        DAX = option yes;
        FS_DAX = option yes;
        VIRTIO_FS = option yes;
        VIRTIO = option yes;
        ZONE_DEVICE = option yes;
      };
    }
  );

  # ── u-root ────────────────────────────────────────────────────────
  # Build framework for minimal initrds.
  uroot = pkgs.buildGoModule rec {
    pname = "u-root";
    version = "0.15.0";
    src = pkgs.fetchFromGitHub {
      owner = "u-root";
      repo = "u-root";
      rev = "v${version}";
      hash = "sha256-5BmM+SHInYngGXmwawKyXTkNIkXsYbCUHyQ8+2blgyU=";
    };
    vendorHash = null;
    doCheck = false; # Some tests invoke /bin/bash
  };

  # ── initrd ────────────────────────────────────────────────────────
  # u-root initrd with snix-init baked in.
  initrd = pkgs.stdenv.mkDerivation {
    name = "snix-initrd.cpio";
    nativeBuildInputs = [pkgs.go];
    # https://github.com/u-root/u-root/issues/2466
    buildCommand = ''
      mkdir -p /tmp/go/src/github.com/u-root/
      cp -R ${uroot.src} /tmp/go/src/github.com/u-root/u-root
      cd /tmp/go/src/github.com/u-root/u-root
      chmod +w .
      cp ${snix-init}/bin/snix-init snix-init

      export HOME=$(mktemp -d)
      export GOROOT="$(go env GOROOT)"

      GO111MODULE=off GOPATH=/tmp/go GOPROXY=off ${uroot}/bin/u-root \
        -files ./snix-init \
        -initcmd "/snix-init" \
        -o $out
    '';
  };

  # ── runVM script ──────────────────────────────────────────────────
  # Start a snix-store virtiofs daemon, then boot cloud-hypervisor.
  #
  # Supports env vars:
  #   CH_NUM_CPUS=2      - number of vCPUs
  #   CH_MEM_SIZE=512M   - memory size
  #   CH_CMDLINE=""      - kernel cmdline (snix.find, snix.shell, snix.run=..., init=...)
  #   ASPEN_TICKET=""    - Aspen cluster ticket for distributed storage
  #                        (when set, starts aspen-snix-bridge and points
  #                         snix-store at it via gRPC Unix socket)
  runVM = pkgs.writers.writeBashBin "run-snix-vm" ''
    set -euo pipefail

    tempdir=$(mktemp -d)
    bridge_pid=""

    cleanup() {
      kill $virtiofsd_pid 2>/dev/null || true
      if [[ -n "''${bridge_pid-}" ]]; then
        kill $bridge_pid 2>/dev/null || true
      fi
      if [[ -n "''${tempdir-}" ]]; then
        chmod -R u+rw "$tempdir" 2>/dev/null || true
        rm -rf "$tempdir"
      fi
    }
    trap cleanup EXIT

    CH_NUM_CPUS="''${CH_NUM_CPUS:-2}"
    CH_MEM_SIZE="''${CH_MEM_SIZE:-512M}"
    CH_CMDLINE="''${CH_CMDLINE:-snix.find}"
    ASPEN_TICKET="''${ASPEN_TICKET:-}"

    if [[ -n "$ASPEN_TICKET" ]]; then
      ${
      if aspen-snix-bridge != null
      then ''
        # Start the gRPC bridge connecting to the Aspen cluster
        echo "Starting aspen-snix-bridge (cluster-connected mode)..."
        ${aspen-snix-bridge}/bin/aspen-snix-bridge \
          --socket "$tempdir/bridge.sock" \
          --ticket "$ASPEN_TICKET" &
        bridge_pid=$!

        # Wait for bridge socket
        for i in $(seq 1 50); do
          [ -e "$tempdir/bridge.sock" ] && break
          sleep 0.1
        done

        if [ ! -e "$tempdir/bridge.sock" ]; then
          echo "ERROR: aspen-snix-bridge socket did not appear"
          exit 1
        fi
        echo "aspen-snix-bridge ready"

        # Point snix-store at the bridge
        export BLOB_SERVICE_ADDR="grpc+unix://$tempdir/bridge.sock"
        export DIRECTORY_SERVICE_ADDR="grpc+unix://$tempdir/bridge.sock"
        export PATH_INFO_SERVICE_ADDR="grpc+unix://$tempdir/bridge.sock"
      ''
      else ''
        echo "ERROR: ASPEN_TICKET set but aspen-snix-bridge not available"
        echo "Pass aspen-snix-bridge to snix-boot to enable cluster mode"
        exit 1
      ''
    }
    else
      # No ticket — use in-memory defaults or caller-provided env vars
      export BLOB_SERVICE_ADDR="''${BLOB_SERVICE_ADDR:-memory:}"
      export DIRECTORY_SERVICE_ADDR="''${DIRECTORY_SERVICE_ADDR:-redb+memory:}"
      export PATH_INFO_SERVICE_ADDR="''${PATH_INFO_SERVICE_ADDR:-redb+memory:}"
    fi

    # Spin up the virtiofs daemon
    ${snix-store}/bin/snix-store virtiofs $tempdir/snix.sock &
    virtiofsd_pid=$!

    # Wait for the socket to exist
    echo "Waiting for VirtioFS socket..."
    for i in $(seq 1 50); do
      [ -e $tempdir/snix.sock ] && break
      sleep 0.1
    done

    if [ ! -e $tempdir/snix.sock ]; then
      echo "ERROR: VirtioFS socket did not appear"
      exit 1
    fi

    echo "Starting Cloud Hypervisor VM (cpus=$CH_NUM_CPUS, mem=$CH_MEM_SIZE)..."

    # Boot cloud-hypervisor
    # num_queues=1 because the vhost-user-fs backend adds a high-priority queue,
    # so 1 normal queue + 1 hiprio = 2 total (which the backend supports).
    ${pkgs.cloud-hypervisor}/bin/cloud-hypervisor \
      --cpus boot=$CH_NUM_CPUS \
      --memory mergeable=on,shared=on,size=$CH_MEM_SIZE \
      --console null \
      --serial tty \
      --kernel ${kernel}/${pkgs.stdenv.hostPlatform.linux-kernel.target} \
      --initramfs ${initrd} \
      --cmdline "console=ttyS0 $CH_CMDLINE" \
      --fs tag=snix,socket=$tempdir/snix.sock,num_queues=1,queue_size=512
  '';
in {
  inherit snix-store snix-init kernel initrd runVM;

  # For use in VM tests
  inherit uroot;

  # Pass-through so callers can check availability
  inherit aspen-snix-bridge;
}
