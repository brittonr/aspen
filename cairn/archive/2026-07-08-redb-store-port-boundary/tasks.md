## Tasks

- [x] [serial] r[molten.modularity.store_ports.explicit_port] Inventoried direct Redb usage and selected chunk-store write/delete planning as the first port extraction domain.
- [x] [serial] r[molten.modularity.store_ports.redb_adapter] Defined `plan_store_write` and `plan_retention_gc` as the store-port planning core while Redb table definitions, open/create, transactions, and error mapping remain shell-owned.
- [x] [serial] r[molten.modularity.store_ports.admission_before_write] Refactored the selected domain boundary so denied requests return no write/delete effects and cannot begin Redb write transactions.
- [x] [parallel] r[molten.modularity.store_ports.tests] Added positive tests for admitted store plans and negative tests for denied, malformed, stale, or unavailable-store inputs.
- [x] [serial] r[molten.modularity.store_ports.tests] Ran `cargo test -p molten-core`, `cargo test --lib`, `cargo fmt --check`, pre-commit, and Cairn validation.
