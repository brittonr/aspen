# NixOS service module

The flake exports `nixosModules.molten` for one explicitly supported local
profile: a bounded control-service loop on the loopback transport. It is not a
WAN, consensus, actor-recovery, or production-readiness profile.

## Supported lifecycle

The module runs a dedicated `molten` system user and a systemd service that:

1. Initializes the exact state root once, if no node config exists.
2. Copies the operator-selected `source-gate-trust.json` into the state root.
3. Runs startup admission and records `startup.preserves`.
4. Runs `molten node serve` in bounded ticks. The supervised loop restarts a
   completed or failed bounded run according to `Restart`.
5. On stop, writes the stop marker and runs `molten node stop`, retaining the
   bounded shutdown and control receipts. Stop-command failures remain failures.
6. Signals the whole owned service control group, including a currently running
   bounded `node serve` and its verifier children; the loop's termination trap
   exits instead of waiting for another source-admission/tick iteration.

The loop is deliberately bounded per run. The service does not claim an
unbounded daemon, continuous lease ownership, or a production SLO.
The unchanged 30-second shutdown budget remains a hard bound. A forced timeout
is not a clean stop; the VM rail checks the service's actual successful stop
result and inactive state, not just the presence of a shutdown receipt.


## Options

| Option | Meaning |
| --- | --- |
| `services.molten.enable` | Enable the service. |
| `services.molten.package` | Exact signed source-binding output containing `bin/molten`, identical to the operator-selected trust document's `output`. Required; the ordinary development `default` package is not interchangeable with the selected output. |
| `services.molten.nodeId` | Node identity used only for the first `node init`. |
| `services.molten.stateDir` | Absolute state directory. Defaults to `/var/lib/molten`. |
| `services.molten.sourceTrustFile` | Operator-selected `source-gate-trust.json`. Required. The file is copied at start; it is not embedded in the Nix store. |
| `services.molten.maxTicks` | Bounded ticks per service run. Defaults to 3600. |
| `services.molten.maxRequestsPerTick` | Bounded requests per tick. Defaults to 16. |
| `services.molten.restartSeconds` | Delay before restarting a failed bounded run. |
| `services.molten.timeoutStartSeconds` | Startup budget for source-closure measurement. |

A custom `stateDir` must already exist and be writable by the `molten` user;
the default `/var/lib/molten` is created and owned through `StateDirectory`.
The service sets `ProtectSystem=strict`, `ProtectHome=true`, `PrivateTmp=true`,
`NoNewPrivileges=true`, and writes only the selected state directory. It does
not open a firewall port or inherit the VM fixture's fixed peers.

## Example

```nix
{
  imports = [ inputs.molten.nixosModules.molten ];

  services.molten = {
    enable = true;
    package = inputs.molten.packages.${pkgs.system}.molten-startup-source-binding;
    nodeId = "node:example";
    sourceTrustFile = "/run/secrets/molten-source-gate-trust.json";
  };
}
```

The selected trust file must name this exact output, its signer, binding, policy, and profile. It is authenticated startup evidence, not authority. A fixture signer or synthetic grant is not a supported operator configuration.

## Validation boundary

`checks.x86_64-linux.molten-module-eval` forces complete NixOS configuration
admission with both explicit package/trust inputs, and rejects either missing
input. It does not assert incidental defaults, boot a VM or prove live behavior.

The opt-in `apps.x86_64-linux.nixos-vm-molten-module` builds a NixOS VM test using the selected binding output. It boots the module, checks startup, restarts and stops the service, then checks absent, stale-binding, wrong-key and wrong-output trust, an altered executing binary, and a privately bind-mounted altered retained policy before new startup evidence. Its fixture signer stays root-only under `/var/lib/molten-trust-fixture`, outside the service state root; only the public trust selection is made accessible to the service. This VM-local signer does not establish independent operator selection. The VM rail is deliberately not a flake check because its cold closure is large.

The declared test profile supplies 8 vCPUs and 8 GiB and copies the complete
closure into a local read-only Nix store image instead of accessing it through
host-store 9p. Every retained member is still measured. Each startup wait and
the service startup deadline remain 3,600 seconds; the aggregate driver bound
stays 7,200 seconds. Substituters remain disabled inside the fixture.

The image is a read-only lower layer. An explicit private writable overlay
permits Nix boot registration and `.links` bookkeeping; source verification
still measures the guest's actual store view. The first
[store-image attempt](../.cairn/changes/observe-real-vm-storage-denial/evidence/module-store-image-readonly-failure-2026-11-16.json)
failed at that registration/signing step because the overlay defaulted off.

An earlier run booted/restarted/stopped the module but failed an incidental negative assertion. A later [2-vCPU/2-GiB attempt](../.cairn/changes/observe-real-vm-storage-denial/evidence/module-vm-startup-timeout-2026-11-16.json), including the shutdown correction, timed out during initial source admission before lifecycle or trust-denial assertions. The new profile captures bounded, best-effort read-only unit/journal/process diagnostics if startup fails; neither the larger resources nor the store image proves the timeout's cause or a passing rerun. A completed exact-package/profile run, successful bounded stop and all trust/source denials remain required before external qualification.

The [completed store-image/overlay attempt](../.cairn/changes/observe-real-vm-storage-denial/evidence/module-store-image-overlay-pass-2026-11-16.json)
passed startup, restart, successful bounded shutdown and all six trust/source
denial controls with the unchanged deadlines. Its exact test derivation and
selected binding are recorded; the private original driver log is content-bound
by BLAKE3. This is fixture execution evidence, not independent operator trust,
qualification of later builds/profiles, or a production startup-latency claim.
