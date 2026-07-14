# Third-party licensing

The root `LICENSE` applies only to repository-owned Molten source. Dependencies, vendored manifest snapshots, generated artifacts containing upstream material, and external references remain under their original terms.

The sparse `vendor/` directory records upstream `Cargo.toml.orig` material rather than changing its license. Current declared families are:

| Paths | Upstream terms recorded by the package or upstream workspace |
|---|---|
| `vendor/datafusion-*` | Apache-2.0; ASF source headers remain authoritative |
| `vendor/iroh*`, `vendor/portmapper-*`, `vendor/postcard`, `vendor/genawaiter-*` | MIT OR Apache-2.0 (legacy `MIT/Apache-2.0` spelling is equivalent package metadata) |
| `vendor/madsim-*`, `vendor/swarm-discovery` | Apache-2.0 |
| `vendor/netlink-packet-core-*` | MIT |
| `vendor/uhlc` | EPL-2.0 OR Apache-2.0 |
| `vendor/nickel-lang-core-0.16.1` | MIT; resolved from the crates.io version record for checksum `51647f09e6e385c140226867c62292f23c31241ee2b4986f3f71a40d48e88a60` |
| `vendor/nostr` (`nostr` 0.44.2) | MIT; resolved from the crates.io version record for checksum `3aa5e3b6a278ed061835fe1ee293b71641e6bf8b401cfe4e1834bbf4ef0a34e1` |

Complete common texts are provided under `THIRD_PARTY_LICENSES/`. Copyright notices and license headers in upstream files must be preserved.

The Nickel and Nostr snapshots inherit their expressions from absent parent workspace manifests, so the table records the exact published version/checksum used to resolve them. Resolution sources: [`nickel-lang-core` 0.16.1](https://crates.io/api/v1/crates/nickel-lang-core/0.16.1) and [`nostr` 0.44.2](https://crates.io/api/v1/crates/nostr/0.44.2), reviewed 2026-07-14. A future version or checksum change MUST be reviewed again rather than inheriting these conclusions. The sparse snapshots are manifest evidence, not complete upstream source bundles; distributors of full upstream source must also preserve all notices shipped by that source.
