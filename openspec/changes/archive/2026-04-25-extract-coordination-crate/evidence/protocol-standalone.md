# V2: Protocol Standalone Evidence

## cargo tree -p aspen-coordination-protocol --edges normal

```
aspen-coordination-protocol v0.1.0 (/home/brittonr/git/aspen/crates/aspen-coordination-protocol)
└── serde v1.0.228
    ├── serde_core v1.0.228
    └── serde_derive v1.0.228 (proc-macro)
        ├── proc-macro2 v1.0.106
        │   └── unicode-ident v1.0.24
        ├── quote v1.0.45
        │   └── proc-macro2 v1.0.106 (*)
        └── syn v2.0.117
            ├── proc-macro2 v1.0.106 (*)
            ├── quote v1.0.45 (*)
            └── unicode-ident v1.0.24
```

## Aspen dependency check

```
OK: no aspen-* packages below protocol crate root
```

## Direct normal dependency check

```
serde
OK: direct normal dependencies are limited to serde
```
