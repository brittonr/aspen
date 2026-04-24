# CLI, dogfood, and handler compatibility evidence

Date: 2026-04-24

## Commands

```console
$ cargo check -p aspen-cli
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
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

warning: `aspen-client-api` (lib) generated 28 warnings
warning: `aspen-hooks-ticket` (lib) generated 2 warnings
warning: unknown lint: `ambient_clock`
   --> crates/aspen-cli/src/bin/aspen-cli/commands/calendar.rs:277:5
    |
277 |     ambient_clock,
    |     ^^^^^^^^^^^^^
    |
    = note: `#[warn(unknown_lints)]` on by default

warning: unknown lint: `ambient_clock`
   --> crates/aspen-cli/src/bin/aspen-cli/commands/cluster.rs:846:5
    |
846 |     ambient_clock,
    |     ^^^^^^^^^^^^^

warning: unknown lint: `ambient_clock`
   --> crates/aspen-cli/src/bin/aspen-cli/commands/cluster.rs:854:5
    |
854 |     ambient_clock,
    |     ^^^^^^^^^^^^^

warning: unknown lint: `ambient_clock`
    --> crates/aspen-cli/src/bin/aspen-cli/commands/federation.rs:1005:5
     |
1005 |     ambient_clock,
     |     ^^^^^^^^^^^^^

warning: unknown lint: `ambient_clock`
   --> crates/aspen-cli/src/bin/aspen-cli/commands/job.rs:468:5
    |
468 |     ambient_clock,
    |     ^^^^^^^^^^^^^

warning: unknown lint: `ambient_clock`
   --> crates/aspen-cli/src/bin/aspen-cli/commands/job.rs:476:5
    |
476 |     ambient_clock,
    |     ^^^^^^^^^^^^^

warning: `aspen-cli` (bin "aspen-cli") generated 6 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.33s
```

```console
$ cargo check -p aspen-dogfood
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
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
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.29s
```

```console
$ cargo check -p aspen-rpc-handlers
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
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
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.33s
```

```console
$ cargo check -p aspen-core-essentials-handler
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
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
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.30s
```

```console
$ cargo check -p aspen-cluster-handler
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
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
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.29s
```

```console
$ cargo check -p aspen-blob-handler
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
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
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.30s
```

```console
$ cargo check -p aspen-forge-handler
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
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
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.34s
```

```console
$ cargo check -p aspen-docs-handler
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
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
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.30s
```

```console
$ cargo check -p aspen-secrets-handler
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
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
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.30s
```

```console
$ cargo check -p aspen-ci-handler
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
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.34s
```

```console
$ cargo check -p aspen-job-handler
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
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
    Checking aspen-jobs-worker-maintenance v0.1.0 (/home/brittonr/git/aspen/crates/aspen-jobs-worker-maintenance)
    Checking aspen-cluster v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster)
    Checking aspen-rpc-core v0.1.0 (/home/brittonr/git/aspen/crates/aspen-rpc-core)
    Checking aspen-job-handler v0.1.0 (/home/brittonr/git/aspen/crates/aspen-job-handler)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.60s
```

```console
$ cargo check -p aspen-nix-handler
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
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
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.30s
```

Status: pass
