# Addressable actor Octet surface

This workspace checks the pure addressable-actor composition with the pinned strict Octet catalog. Nix copies the product module into this focused workspace and replaces product tests with an empty module. The result is source-shape evidence only. It does not prove runtime, storage, transport, authority, or release correctness.

Nix also mounts the authoritative `crates/molten-core/src/system_extension/generation/mod.rs` implementation at `src/system_extension/generation/mod.rs`. The focused lifecycle facade uses its nominal `Generation` and `Incarnation` types rather than raw-integer substitutes, so actor binding validation sees the production generation semantics.

The private facade mounts that file as `counters`, keeping both nominal roles under one neutral namespace. Their public names and production implementations are unchanged; this avoids repeating `generation::Generation` without disabling the strict path-name lint.
