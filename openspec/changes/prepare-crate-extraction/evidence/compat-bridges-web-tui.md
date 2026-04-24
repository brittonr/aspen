# Bridge, gateway, web, and TUI compatibility evidence

Date: 2026-04-24

## Commands

```console
$ cargo check -p aspen-cluster-bridges
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Checking aspen-hlc v0.1.0 (/home/brittonr/git/aspen/crates/aspen-hlc)
    Checking aspen-storage-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-storage-types)
    Checking aspen-time v0.1.0 (/home/brittonr/git/aspen/crates/aspen-time)
    Checking aspen-layer v0.1.0 (/home/brittonr/git/aspen/crates/aspen-layer)
    Checking aspen-redb-storage v0.1.0 (/home/brittonr/git/aspen/crates/aspen-redb-storage)
    Checking aspen-trust v0.1.0 (/home/brittonr/git/aspen/crates/aspen-trust)
warning: unknown lint: `acronym_style`
  --> openraft/openraft/src/error/mod.rs:80:13
   |
80 |     #[allow(acronym_style, reason = "preserve established public error variant spelling")]
   |             ^^^^^^^^^^^^^
   |
   = note: `#[warn(unknown_lints)]` on by default

warning: unknown lint: `acronym_style`
   --> openraft/openraft/src/error/mod.rs:290:9
    |
290 | #[allow(acronym_style, reason = "preserve established public error type spelling")]
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
 --> openraft/openraft/src/network/rpc_option.rs:8:9
  |
8 | #[allow(acronym_style, reason = "preserve established public transport option spelling")]
  |         ^^^^^^^^^^^^^

warning: method `committed_leader_id` is never used
  --> openraft/openraft/src/log_id/option_raft_log_id_ext.rs:22:8
   |
 8 | pub(crate) trait OptionRaftLogIdExt<C>
   |                  ------------------ method in this trait
...
22 |     fn committed_leader_id(&self) -> Option<&CommittedLeaderIdOf<C>>;
   |        ^^^^^^^^^^^^^^^^^^^
   |
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: `openraft` (lib) generated 4 warnings
    Checking aspen-cluster-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-types)
    Checking aspen-auth-core v0.1.0 (/home/brittonr/git/aspen/crates/aspen-auth-core)
    Checking aspen-hooks-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-hooks-types)
    Checking aspen-traits v0.1.0 (/home/brittonr/git/aspen/crates/aspen-traits)
    Checking aspen-ticket v0.1.0 (/home/brittonr/git/aspen/crates/aspen-ticket)
    Checking aspen-hooks-ticket v0.1.0 (/home/brittonr/git/aspen/crates/aspen-hooks-ticket)
warning: constant `MINUTES_PER_HOUR` is never used
   --> crates/aspen-hooks-ticket/src/lib.rs:101:7
    |
101 | const MINUTES_PER_HOUR: u64 = 60;
    |       ^^^^^^^^^^^^^^^^
    |
    = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: constant `HOURS_PER_DAY` is never used
   --> crates/aspen-hooks-ticket/src/lib.rs:104:7
    |
104 | const HOURS_PER_DAY: u64 = 24;
    |       ^^^^^^^^^^^^^

    Checking aspen-client-api v0.1.0 (/home/brittonr/git/aspen/crates/aspen-client-api)
    Checking aspen-core v0.1.0 (/home/brittonr/git/aspen/crates/aspen-core)
warning: `aspen-hooks-ticket` (lib) generated 2 warnings
    Checking aspen-core-shell v0.1.0 (/home/brittonr/git/aspen/crates/aspen-core-shell)
    Checking aspen-raft-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft-types)
    Checking aspen-sharding v0.1.0 (/home/brittonr/git/aspen/crates/aspen-sharding)
    Checking aspen-coordination v0.1.0 (/home/brittonr/git/aspen/crates/aspen-coordination)
    Checking aspen-auth v0.1.0 (/home/brittonr/git/aspen/crates/aspen-auth)
    Checking aspen-transport v0.1.0 (/home/brittonr/git/aspen/crates/aspen-transport)
    Checking aspen-raft-network v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft-network)
    Checking aspen-jobs v0.1.0 (/home/brittonr/git/aspen/crates/aspen-jobs)
warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:201:9
    |
201 |         acronym_style,
    |         ^^^^^^^^^^^^^
    |
    = note: `#[warn(unknown_lints)]` on by default

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:212:9
    |
212 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:222:9
    |
222 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:233:9
    |
233 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:243:9
    |
243 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:249:9
    |
249 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:259:9
    |
259 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:270:9
    |
270 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:334:9
    |
334 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:340:9
    |
340 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1480:9
     |
1480 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1496:9
     |
1496 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1510:9
     |
1510 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1526:9
     |
1526 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1540:9
     |
1540 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1552:9
     |
1552 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1566:9
     |
1566 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1582:9
     |
1582 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4270:9
     |
4270 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4277:9
     |
4277 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4284:9
     |
4284 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4291:9
     |
4291 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4298:9
     |
4298 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4305:9
     |
4305 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4312:9
     |
4312 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4319:9
     |
4319 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4359:9
     |
4359 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4366:9
     |
4366 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: `aspen-client-api` (lib) generated 28 warnings
    Checking aspen-client v0.1.0 (/home/brittonr/git/aspen/crates/aspen-client)
    Checking aspen-raft v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft)
    Checking aspen-hooks v0.1.0 (/home/brittonr/git/aspen/crates/aspen-hooks)
    Checking aspen-cluster-bridges v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-bridges)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.26s
```

```console
$ cargo check -p aspen-snix-bridge
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Checking aspen-hlc v0.1.0 (/home/brittonr/git/aspen/crates/aspen-hlc)
    Checking aspen-cluster-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-types)
    Checking aspen-auth-core v0.1.0 (/home/brittonr/git/aspen/crates/aspen-auth-core)
    Checking aspen-trust v0.1.0 (/home/brittonr/git/aspen/crates/aspen-trust)
warning: unknown lint: `acronym_style`
  --> openraft/openraft/src/error/mod.rs:80:13
   |
80 |     #[allow(acronym_style, reason = "preserve established public error variant spelling")]
   |             ^^^^^^^^^^^^^
   |
   = note: `#[warn(unknown_lints)]` on by default

warning: unknown lint: `acronym_style`
   --> openraft/openraft/src/error/mod.rs:290:9
    |
290 | #[allow(acronym_style, reason = "preserve established public error type spelling")]
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
 --> openraft/openraft/src/network/rpc_option.rs:8:9
  |
8 | #[allow(acronym_style, reason = "preserve established public transport option spelling")]
  |         ^^^^^^^^^^^^^

warning: method `committed_leader_id` is never used
  --> openraft/openraft/src/log_id/option_raft_log_id_ext.rs:22:8
   |
 8 | pub(crate) trait OptionRaftLogIdExt<C>
   |                  ------------------ method in this trait
...
22 |     fn committed_leader_id(&self) -> Option<&CommittedLeaderIdOf<C>>;
   |        ^^^^^^^^^^^^^^^^^^^
   |
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: `openraft` (lib) generated 4 warnings
    Checking aspen-traits v0.1.0 (/home/brittonr/git/aspen/crates/aspen-traits)
    Checking aspen-ticket v0.1.0 (/home/brittonr/git/aspen/crates/aspen-ticket)
    Checking aspen-client-api v0.1.0 (/home/brittonr/git/aspen/crates/aspen-client-api)
    Checking aspen-core v0.1.0 (/home/brittonr/git/aspen/crates/aspen-core)
    Checking aspen-testing-core v0.1.0 (/home/brittonr/git/aspen/crates/aspen-testing-core)
    Checking aspen-testing-fixtures v0.1.0 (/home/brittonr/git/aspen/crates/aspen-testing-fixtures)
    Checking aspen-core-shell v0.1.0 (/home/brittonr/git/aspen/crates/aspen-core-shell)
    Checking aspen-sharding v0.1.0 (/home/brittonr/git/aspen/crates/aspen-sharding)
    Checking aspen-raft-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft-types)
    Checking aspen-auth v0.1.0 (/home/brittonr/git/aspen/crates/aspen-auth)
    Checking aspen-testing v0.1.0 (/home/brittonr/git/aspen/crates/aspen-testing)
    Checking aspen-transport v0.1.0 (/home/brittonr/git/aspen/crates/aspen-transport)
warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:201:9
    |
201 |         acronym_style,
    |         ^^^^^^^^^^^^^
    |
    = note: `#[warn(unknown_lints)]` on by default

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:212:9
    |
212 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:222:9
    |
222 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:233:9
    |
233 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:243:9
    |
243 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:249:9
    |
249 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:259:9
    |
259 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:270:9
    |
270 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:334:9
    |
334 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:340:9
    |
340 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1480:9
     |
1480 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1496:9
     |
1496 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1510:9
     |
1510 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1526:9
     |
1526 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1540:9
     |
1540 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1552:9
     |
1552 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1566:9
     |
1566 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1582:9
     |
1582 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4270:9
     |
4270 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4277:9
     |
4277 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4284:9
     |
4284 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4291:9
     |
4291 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4298:9
     |
4298 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4305:9
     |
4305 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4312:9
     |
4312 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4319:9
     |
4319 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4359:9
     |
4359 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4366:9
     |
4366 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: `aspen-client-api` (lib) generated 28 warnings
    Checking aspen-blob v0.1.0 (/home/brittonr/git/aspen/crates/aspen-blob)
    Checking aspen-cache v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cache)
    Checking aspen-client v0.1.0 (/home/brittonr/git/aspen/crates/aspen-client)
    Checking aspen-snix v0.1.0 (/home/brittonr/git/aspen/crates/aspen-snix)
    Checking aspen-snix-bridge v0.1.0 (/home/brittonr/git/aspen/crates/aspen-snix-bridge)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.37s
```

```console
$ cargo check -p aspen-nix-cache-gateway
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:201:9
    |
201 |         acronym_style,
    |         ^^^^^^^^^^^^^
    |
    = note: `#[warn(unknown_lints)]` on by default

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:212:9
    |
212 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:222:9
    |
222 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:233:9
    |
233 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:243:9
    |
243 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:249:9
    |
249 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:259:9
    |
259 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:270:9
    |
270 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:334:9
    |
334 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:340:9
    |
340 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1480:9
     |
1480 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1496:9
     |
1496 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1510:9
     |
1510 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1526:9
     |
1526 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1540:9
     |
1540 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1552:9
     |
1552 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1566:9
     |
1566 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1582:9
     |
1582 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4270:9
     |
4270 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4277:9
     |
4277 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4284:9
     |
4284 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4291:9
     |
4291 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4298:9
     |
4298 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4305:9
     |
4305 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4312:9
     |
4312 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4319:9
     |
4319 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4359:9
     |
4359 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4366:9
     |
4366 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: `aspen-client-api` (lib) generated 28 warnings
    Checking aspen-core-shell v0.1.0 (/home/brittonr/git/aspen/crates/aspen-core-shell)
    Checking aspen-blob v0.1.0 (/home/brittonr/git/aspen/crates/aspen-blob)
warning: unknown lint: `acronym_style`
  --> openraft/openraft/src/error/mod.rs:80:13
   |
80 |     #[allow(acronym_style, reason = "preserve established public error variant spelling")]
   |             ^^^^^^^^^^^^^
   |
   = note: `#[warn(unknown_lints)]` on by default

warning: unknown lint: `acronym_style`
   --> openraft/openraft/src/error/mod.rs:290:9
    |
290 | #[allow(acronym_style, reason = "preserve established public error type spelling")]
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
 --> openraft/openraft/src/network/rpc_option.rs:8:9
  |
8 | #[allow(acronym_style, reason = "preserve established public transport option spelling")]
  |         ^^^^^^^^^^^^^

warning: method `committed_leader_id` is never used
  --> openraft/openraft/src/log_id/option_raft_log_id_ext.rs:22:8
   |
 8 | pub(crate) trait OptionRaftLogIdExt<C>
   |                  ------------------ method in this trait
...
22 |     fn committed_leader_id(&self) -> Option<&CommittedLeaderIdOf<C>>;
   |        ^^^^^^^^^^^^^^^^^^^
   |
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

    Checking aspen-trust v0.1.0 (/home/brittonr/git/aspen/crates/aspen-trust)
warning: `openraft` (lib) generated 4 warnings
    Checking aspen-ticket v0.1.0 (/home/brittonr/git/aspen/crates/aspen-ticket)
    Checking aspen-raft-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft-types)
    Checking aspen-auth v0.1.0 (/home/brittonr/git/aspen/crates/aspen-auth)
    Checking aspen-transport v0.1.0 (/home/brittonr/git/aspen/crates/aspen-transport)
    Checking aspen-cache v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cache)
    Checking aspen-snix v0.1.0 (/home/brittonr/git/aspen/crates/aspen-snix)
    Checking aspen-client v0.1.0 (/home/brittonr/git/aspen/crates/aspen-client)
    Checking aspen-nix-cache-gateway v0.1.0 (/home/brittonr/git/aspen/crates/aspen-nix-cache-gateway)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.27s
```

```console
$ cargo check -p aspen-h3-proxy
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Checking aspen-cluster-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-types)
    Checking aspen-hlc v0.1.0 (/home/brittonr/git/aspen/crates/aspen-hlc)
    Checking aspen-trust v0.1.0 (/home/brittonr/git/aspen/crates/aspen-trust)
warning: unknown lint: `acronym_style`
  --> openraft/openraft/src/error/mod.rs:80:13
   |
80 |     #[allow(acronym_style, reason = "preserve established public error variant spelling")]
   |             ^^^^^^^^^^^^^
   |
   = note: `#[warn(unknown_lints)]` on by default

warning: unknown lint: `acronym_style`
   --> openraft/openraft/src/error/mod.rs:290:9
    |
290 | #[allow(acronym_style, reason = "preserve established public error type spelling")]
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
 --> openraft/openraft/src/network/rpc_option.rs:8:9
  |
8 | #[allow(acronym_style, reason = "preserve established public transport option spelling")]
  |         ^^^^^^^^^^^^^

warning: method `committed_leader_id` is never used
  --> openraft/openraft/src/log_id/option_raft_log_id_ext.rs:22:8
   |
 8 | pub(crate) trait OptionRaftLogIdExt<C>
   |                  ------------------ method in this trait
...
22 |     fn committed_leader_id(&self) -> Option<&CommittedLeaderIdOf<C>>;
   |        ^^^^^^^^^^^^^^^^^^^
   |
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: `openraft` (lib) generated 4 warnings
    Checking aspen-auth-core v0.1.0 (/home/brittonr/git/aspen/crates/aspen-auth-core)
    Checking aspen-traits v0.1.0 (/home/brittonr/git/aspen/crates/aspen-traits)
    Checking aspen-core v0.1.0 (/home/brittonr/git/aspen/crates/aspen-core)
    Checking aspen-core-shell v0.1.0 (/home/brittonr/git/aspen/crates/aspen-core-shell)
    Checking aspen-sharding v0.1.0 (/home/brittonr/git/aspen/crates/aspen-sharding)
    Checking aspen-raft-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft-types)
    Checking aspen-auth v0.1.0 (/home/brittonr/git/aspen/crates/aspen-auth)
    Checking aspen-transport v0.1.0 (/home/brittonr/git/aspen/crates/aspen-transport)
    Checking aspen-h3-proxy v0.1.0 (/home/brittonr/git/aspen/crates/aspen-h3-proxy)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.03s
```

```console
$ cargo check -p aspen-forge-web
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Checking aspen-trust v0.1.0 (/home/brittonr/git/aspen/crates/aspen-trust)
warning: unknown lint: `acronym_style`
  --> openraft/openraft/src/error/mod.rs:80:13
   |
80 |     #[allow(acronym_style, reason = "preserve established public error variant spelling")]
   |             ^^^^^^^^^^^^^
   |
   = note: `#[warn(unknown_lints)]` on by default

warning: unknown lint: `acronym_style`
   --> openraft/openraft/src/error/mod.rs:290:9
    |
290 | #[allow(acronym_style, reason = "preserve established public error type spelling")]
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
 --> openraft/openraft/src/network/rpc_option.rs:8:9
  |
8 | #[allow(acronym_style, reason = "preserve established public transport option spelling")]
  |         ^^^^^^^^^^^^^

warning: method `committed_leader_id` is never used
  --> openraft/openraft/src/log_id/option_raft_log_id_ext.rs:22:8
   |
 8 | pub(crate) trait OptionRaftLogIdExt<C>
   |                  ------------------ method in this trait
...
22 |     fn committed_leader_id(&self) -> Option<&CommittedLeaderIdOf<C>>;
   |        ^^^^^^^^^^^^^^^^^^^
   |
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: `openraft` (lib) generated 4 warnings
    Checking aspen-cluster-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-types)
    Checking aspen-auth-core v0.1.0 (/home/brittonr/git/aspen/crates/aspen-auth-core)
    Checking aspen-traits v0.1.0 (/home/brittonr/git/aspen/crates/aspen-traits)
    Checking aspen-ticket v0.1.0 (/home/brittonr/git/aspen/crates/aspen-ticket)
    Checking aspen-client-api v0.1.0 (/home/brittonr/git/aspen/crates/aspen-client-api)
    Checking aspen-core v0.1.0 (/home/brittonr/git/aspen/crates/aspen-core)
    Checking aspen-core-shell v0.1.0 (/home/brittonr/git/aspen/crates/aspen-core-shell)
    Checking aspen-sharding v0.1.0 (/home/brittonr/git/aspen/crates/aspen-sharding)
    Checking aspen-raft-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft-types)
    Checking aspen-auth v0.1.0 (/home/brittonr/git/aspen/crates/aspen-auth)
    Checking aspen-transport v0.1.0 (/home/brittonr/git/aspen/crates/aspen-transport)
    Checking aspen-h3-proxy v0.1.0 (/home/brittonr/git/aspen/crates/aspen-h3-proxy)
warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:201:9
    |
201 |         acronym_style,
    |         ^^^^^^^^^^^^^
    |
    = note: `#[warn(unknown_lints)]` on by default

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:212:9
    |
212 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:222:9
    |
222 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:233:9
    |
233 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:243:9
    |
243 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:249:9
    |
249 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:259:9
    |
259 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:270:9
    |
270 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:334:9
    |
334 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:340:9
    |
340 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1480:9
     |
1480 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1496:9
     |
1496 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1510:9
     |
1510 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1526:9
     |
1526 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1540:9
     |
1540 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1552:9
     |
1552 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1566:9
     |
1566 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1582:9
     |
1582 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4270:9
     |
4270 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4277:9
     |
4277 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4284:9
     |
4284 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4291:9
     |
4291 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4298:9
     |
4298 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4305:9
     |
4305 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4312:9
     |
4312 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4319:9
     |
4319 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4359:9
     |
4359 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4366:9
     |
4366 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: `aspen-client-api` (lib) generated 28 warnings
    Checking aspen-client v0.1.0 (/home/brittonr/git/aspen/crates/aspen-client)
    Checking aspen-forge-web v0.1.0 (/home/brittonr/git/aspen/crates/aspen-forge-web)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.80s
```

```console
$ cargo check -p aspen-tui
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Checking aspen-trust v0.1.0 (/home/brittonr/git/aspen/crates/aspen-trust)
    Checking aspen-cluster-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-types)
    Checking aspen-auth-core v0.1.0 (/home/brittonr/git/aspen/crates/aspen-auth-core)
warning: unknown lint: `acronym_style`
  --> openraft/openraft/src/error/mod.rs:80:13
   |
80 |     #[allow(acronym_style, reason = "preserve established public error variant spelling")]
   |             ^^^^^^^^^^^^^
   |
   = note: `#[warn(unknown_lints)]` on by default

warning: unknown lint: `acronym_style`
   --> openraft/openraft/src/error/mod.rs:290:9
    |
290 | #[allow(acronym_style, reason = "preserve established public error type spelling")]
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
 --> openraft/openraft/src/network/rpc_option.rs:8:9
  |
8 | #[allow(acronym_style, reason = "preserve established public transport option spelling")]
  |         ^^^^^^^^^^^^^

warning: method `committed_leader_id` is never used
  --> openraft/openraft/src/log_id/option_raft_log_id_ext.rs:22:8
   |
 8 | pub(crate) trait OptionRaftLogIdExt<C>
   |                  ------------------ method in this trait
...
22 |     fn committed_leader_id(&self) -> Option<&CommittedLeaderIdOf<C>>;
   |        ^^^^^^^^^^^^^^^^^^^
   |
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

    Checking aspen-ci-core v0.1.0 (/home/brittonr/git/aspen/crates/aspen-ci-core)
warning: `openraft` (lib) generated 4 warnings
    Checking aspen-traits v0.1.0 (/home/brittonr/git/aspen/crates/aspen-traits)
    Checking aspen-ticket v0.1.0 (/home/brittonr/git/aspen/crates/aspen-ticket)
    Checking aspen-client-api v0.1.0 (/home/brittonr/git/aspen/crates/aspen-client-api)
    Checking aspen-core v0.1.0 (/home/brittonr/git/aspen/crates/aspen-core)
    Checking aspen-core-shell v0.1.0 (/home/brittonr/git/aspen/crates/aspen-core-shell)
    Checking aspen-sharding v0.1.0 (/home/brittonr/git/aspen/crates/aspen-sharding)
    Checking aspen-raft-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft-types)
    Checking aspen-auth v0.1.0 (/home/brittonr/git/aspen/crates/aspen-auth)
    Checking aspen-transport v0.1.0 (/home/brittonr/git/aspen/crates/aspen-transport)
warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:201:9
    |
201 |         acronym_style,
    |         ^^^^^^^^^^^^^
    |
    = note: `#[warn(unknown_lints)]` on by default

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:212:9
    |
212 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:222:9
    |
222 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:233:9
    |
233 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:243:9
    |
243 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:249:9
    |
249 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:259:9
    |
259 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:270:9
    |
270 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:334:9
    |
334 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
   --> crates/aspen-client-api/src/messages/coordination.rs:340:9
    |
340 |         acronym_style,
    |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1480:9
     |
1480 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1496:9
     |
1496 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1510:9
     |
1510 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1526:9
     |
1526 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1540:9
     |
1540 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1552:9
     |
1552 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1566:9
     |
1566 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:1582:9
     |
1582 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4270:9
     |
4270 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4277:9
     |
4277 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4284:9
     |
4284 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4291:9
     |
4291 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4298:9
     |
4298 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4305:9
     |
4305 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4312:9
     |
4312 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4319:9
     |
4319 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4359:9
     |
4359 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: unknown lint: `acronym_style`
    --> crates/aspen-client-api/src/messages/mod.rs:4366:9
     |
4366 |         acronym_style,
     |         ^^^^^^^^^^^^^

warning: `aspen-client-api` (lib) generated 28 warnings
    Checking aspen-client v0.1.0 (/home/brittonr/git/aspen/crates/aspen-client)
    Checking aspen-tui v0.1.0 (/home/brittonr/git/aspen/crates/aspen-tui)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.82s
```

Status: pass
