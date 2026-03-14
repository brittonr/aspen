# Verus formal verification toolchain.
#
# Verus build and the verus devShell remain in flake.nix for now —
# the verus binary is used by the default devShell, apps, and checks.
#
# Migration plan: extract verus + z3 builds here, expose via options,
# then have dev.nix and apps.nix consume them.
{...}: {}
