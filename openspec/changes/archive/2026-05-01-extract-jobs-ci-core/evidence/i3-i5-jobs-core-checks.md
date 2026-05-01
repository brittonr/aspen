# I3/I5 aspen-jobs-core compile and test evidence

## cargo tree -p aspen-jobs-core -e normal --depth 2
```text
aspen-jobs-core v0.1.0 (/home/brittonr/git/aspen/crates/aspen-jobs-core)
├── chrono v0.4.44
│   ├── iana-time-zone v0.1.65
│   ├── num-traits v0.2.19
│   └── serde v1.0.228
├── serde v1.0.228 (*)
├── serde_json v1.0.149
│   ├── itoa v1.0.17
│   ├── memchr v2.8.0
│   ├── serde_core v1.0.228
│   └── zmij v1.0.21
├── thiserror v2.0.18
│   └── thiserror-impl v2.0.18 (proc-macro)
└── uuid v1.22.0
    ├── getrandom v0.4.2
    └── serde_core v1.0.228
```

## cargo check -p aspen-jobs-core
```text
```

## cargo test -p aspen-jobs-core
```text

running 3 tests
test tests::retry_transition_respects_policy ... ok
test tests::status_helpers_match_runtime_contract ... ok
test tests::job_spec_roundtrips_json ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```
