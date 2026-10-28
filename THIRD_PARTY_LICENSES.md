# Third-party licensing

The root `LICENSE` applies only to repository-owned Molten source. Dependencies, vendored manifest snapshots, generated artifacts containing upstream material, and external references remain under their original terms.

The sparse `vendor/` directory records upstream `Cargo.toml.orig` material rather than changing its license. Current declared families are:

| Paths | Upstream terms recorded by the package or upstream workspace |
|---|---|
| Cargo packages `artifact-auth-core`, `artifact-auth-ed25519`, and `artifact-binding-core` from `OnixResearch/onix-artifact` revision `c932138d880ddf4c2967f4c024b489b5c0022bf1` | MIT OR Apache-2.0 |
| Cargo packages `hardened-wasmtime-runtime` and `hardened-wasmtime-shell` from `rad://z3hRCegTsS8jpJVgxYfb9psEzxHpG/z6MksnXbFoE8zkCkGWhHc8zuxpnEUhrJHv2KECRV4GSv9gkx` revision `cf4a1913f7f2ca7eb7f655663e87b8b72d0c506c` | MIT |
| Cargo packages `mantle-wasm-consumer-verifier` and `crunch-wasm-component-core` from `OnixResearch/mantle` revision `cf7623b81a63e3df9b5d69ad812c4a63ab78c1e0` | AGPL-3.0-or-later |
| `tests/fixtures/wasm-component-runtime/kamacite-mantle-hello-bundle.json`, copied unchanged from `crates/mantle-wasm-consumer-verifier/tests/fixtures/` at Mantle revision `c99556e5c91dfd13fcc261dc451f471359d47c38` | AGPL-3.0-or-later |
| `tests/fixtures/wasm-component-runtime/producer-tree-handoff/`, unchanged required publication members and bundle from the actual Mantle pipeline at revision `c99556e5c91dfd13fcc261dc451f471359d47c38`; extracted from retained `producer-snapshot.zip` (BLAKE3 `c701aa6901f614842cb2ec33d7a9b631e31f6471703bc82ee1d1ea9fce7bc7ed`) in `.cairn/changes/adopt-shared-wasm-component-runtime/evidence/2026-10-27-producer-handoff/`; unrelated reports omitted, original bytes/logical paths/digests unchanged | AGPL-3.0-or-later |
| Cargo packages `bounded-tree-core` and `bounded-tree-cap` from `https://seed.radicle.garden/zqhtZvsteJhxCJE96dMAZSZ9y1PX.git` revision `250f3b25d0933c2fa106aa82d1cc17a496716f06`, transitively selected by Mantle's source-closure verifier | AGPL-3.0-or-later |
| `.cairn/changes/adopt-shared-wasm-component-runtime/evidence/2026-10-27-source-tree-intake/immutable-producer-snapshot.zip`, unchanged actual publication from Mantle's fixture and pipeline at revision `cf7623b81a63e3df9b5d69ad812c4a63ab78c1e0` | AGPL-3.0-or-later |
| Cargo packages `flux-profiler`, `flux-profiler-macros`, `flux-communication`, `flux-timing`, `flux-utils`, `type-hash`, and `type-hash-derive` from `gattaca-com/flux` revision `2a1916465ae6649aebef3758233cfea98e5d33db` | Apache-2.0 AND MIT |
| `vendor/datafusion-*` | Apache-2.0; ASF source headers remain authoritative |
| `vendor/iroh*`, `vendor/portmapper-*`, `vendor/postcard`, `vendor/genawaiter-*` | MIT OR Apache-2.0 (legacy `MIT/Apache-2.0` spelling is equivalent package metadata) |
| `vendor/madsim-*`, `vendor/swarm-discovery` | Apache-2.0 |
| `vendor/netlink-packet-core-*` | MIT |
| `vendor/uhlc` | EPL-2.0 OR Apache-2.0 |
| `vendor/nickel-lang-core-0.16.1` | MIT; resolved from the crates.io version record for checksum `51647f09e6e385c140226867c62292f23c31241ee2b4986f3f71a40d48e88a60` |
| `vendor/nostr` (`nostr` 0.44.2) | MIT; resolved from the crates.io version record for checksum `3aa5e3b6a278ed061835fe1ee293b71641e6bf8b401cfe4e1834bbf4ef0a34e1` |

Complete common texts are provided under `THIRD_PARTY_LICENSES/`. Copyright notices and license headers in upstream files must be preserved.

The Nickel and Nostr snapshots inherit their expressions from absent parent workspace manifests, so the table records the exact published version/checksum used to resolve them. Resolution sources: [`nickel-lang-core` 0.16.1](https://crates.io/api/v1/crates/nickel-lang-core/0.16.1) and [`nostr` 0.44.2](https://crates.io/api/v1/crates/nostr/0.44.2), reviewed 2026-07-14. A future version or checksum change MUST be reviewed again rather than inheriting these conclusions. The sparse snapshots are manifest evidence, not complete upstream source bundles; distributors of full upstream source must also preserve all notices shipped by that source.
