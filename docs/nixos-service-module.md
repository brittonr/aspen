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
   bounded shutdown and control receipts.

The loop is deliberately bounded per run. The service does not claim an
unbounded daemon, continuous lease ownership, or a production SLO.

## Options

| Option | Meaning |
| --- | --- |
| `services.molten.enable` | Enable the service. |
| `services.molten.package` | Exact Molten package. Required; the module does not assume a nixpkgs attribute. |
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
    package = inputs.molten.packages.${pkgs.system}.default;
    nodeId = "node:example";
    sourceTrustFile = "/run/secrets/molten-source-gate-trust.json";
  };
}
```

The selected trust file is authenticated startup evidence, not authority. A
fixture signer or synthetic grant is not a supported operator configuration.

## Validation boundary

`checks.x86_64-linux.molten-module-eval` evaluates the module, the required
package and trust inputs, the service user, and the generated start and loop
scripts. It does not boot a VM or prove live node behavior.

The opt-in `apps.x86_64-linux.nixos-vm-molten-module` builds a NixOS VM test
that boots the module, checks startup, restarts the service, stops it, and
checks absent-trust failure. It is deliberately not a flake check because its
cold closure is large.

The test VM disables substituters and raises the driver timeout because each
start remeasures the retained build closure with `nix hash path`; the default
one-hour driver timeout is shorter than two measured starts on a small VM. A
run booted the module, wrote startup evidence, restarted it, wrote shutdown
evidence, and denied an absent trust file. Its final assertion expected the
unit to be `failed`, but `Restart=on-failure` leaves it restarting; the test now
asserts that the unit is not active. The test also now covers a stale source
selection and a wrong signer, both before startup evidence. The corrected full
run was not retained because the host Nix store remained unresponsive to
evaluation. Boot, restart, shutdown, absent/wrong/stale trust denial, and the
exact package/profile result remain required before the module is treated as
externally qualified.
