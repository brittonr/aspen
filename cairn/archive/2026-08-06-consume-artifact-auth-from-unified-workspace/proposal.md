# Change: Consume artifact authentication from the unified Artifact workspace

## Why

Artifact owns authentication and binding packages in one workspace. Molten still uses the predecessor authentication source and a separate Artifact binding input. This split permits mixed source identity inside one product graph.

## What changes

- Pin all three consumed Artifact packages to revision `c932138d880ddf4c2967f4c024b489b5c0022bf1`.
- Replace separate authentication and binding Nix inputs with one Artifact input.
- Align Cargo, Nix, release policy, and both unit2nix plans.
- Add positive and negative source-admission checks and typed migration evidence.
- Keep the predecessor Radicle receipt as historical evidence only.
- Preserve Molten-owned runtime, identity, capability, transport, and release authority.

## Impact

The change affects the `artifact-auth-adoption` specification, source pins, generated locks and plans, release policy, evidence, and documentation. It does not change Rust behavior or authentication APIs.
