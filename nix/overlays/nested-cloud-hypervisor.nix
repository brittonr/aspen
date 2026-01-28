# Overlay to wrap cloud-hypervisor with nested virtualization support
#
# Cloud Hypervisor's --cpus argument cannot be specified multiple times, and
# microvm.nix hardcodes `--cpus boot=N`. This overlay wraps the cloud-hypervisor
# binary to intercept the --cpus argument and append `,nested=on` to enable
# nested virtualization (required for running VMs inside VMs).
#
# Usage in nixpkgs import:
#   pkgs = import nixpkgs {
#     system = "x86_64-linux";
#     overlays = [ (import ./nix/overlays/nested-cloud-hypervisor.nix) ];
#   };
final: prev: {
  cloud-hypervisor = prev.cloud-hypervisor.overrideAttrs (oldAttrs: {
    postInstall =
      (oldAttrs.postInstall or "")
      + ''
        # Wrap cloud-hypervisor to add nested=on to --cpus argument
        mv $out/bin/cloud-hypervisor $out/bin/.cloud-hypervisor-unwrapped

        # Create wrapper script
        cat > $out/bin/cloud-hypervisor << 'WRAPPER_EOF'
        #!/bin/sh
        # Wrapper to enable nested virtualization by appending nested=on to --cpus
        # This is required for running Cloud Hypervisor inside Cloud Hypervisor VMs

        self_dir="$(dirname "$(readlink -f "$0")")"
        real_binary="$self_dir/.cloud-hypervisor-unwrapped"

        # Process arguments, appending nested=on to --cpus value
        args=()
        while [ $# -gt 0 ]; do
          case "$1" in
            --cpus)
              args+=("--cpus")
              shift
              if [ $# -gt 0 ]; then
                # Append nested=on to the cpus argument value
                args+=("$1,nested=on")
                shift
              fi
              ;;
            *)
              args+=("$1")
              shift
              ;;
          esac
        done

        exec "$real_binary" "''${args[@]}"
        WRAPPER_EOF

        chmod +x $out/bin/cloud-hypervisor
      '';
  });
}
