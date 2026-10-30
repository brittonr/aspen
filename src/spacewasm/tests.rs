fn module(memory: &str, extra: &str, body: &str) -> Vec<u8> {
    wat::parse_str(format!(
        r#"(module
        {extra}
        (memory (export "memory") {memory})
        (func (export "molten_alloc") (param i32) (result i32) i32.const 0)
        (func (export "molten_dealloc") (param i32 i32))
        (func (export "molten_run") (param i32 i32) (result i64) {body})
    )"#
    ))
    .expect("valid fixture WAT")
}

#[test]
fn rejects_unbounded_memory_growth_and_start_before_execution() {
    let profile = super::super::profile::Profile::default();
    let cases = [
        module("1", "", "i64.const 0"),
        module("1 257", "", "i64.const 0"),
        module("1 1", "", "i32.const 1 memory.grow drop i64.const 0"),
        module("1 1", "(func $start) (start $start)", "i64.const 0"),
        module("1 1", "(table 1 1 funcref)", "i64.const 0"),
    ];
    for bytes in cases {
        assert!(super::inspect(&bytes, &profile).is_err());
    }
}

#[test]
fn resolves_signatures_and_denies_ambient_or_wrong_hostcalls() {
    let profile = super::super::profile::Profile::default();
    let admitted =
        module("1 1", r#"(import "molten:hostcall" "send" (func (param i32 i32) (result i64)))"#, "i64.const 0");
    assert_eq!(super::inspect(&admitted, &profile).expect("admitted").hostcalls, vec!["send"]);
    for import in [
        r#"(import "wasi_snapshot_preview1" "random_get" (func (param i32 i32) (result i64)))"#,
        r#"(import "molten:hostcall" "send" (func (param i64) (result i64)))"#,
        r#"(import "molten:hostcall" "clock" (func (param i32 i32) (result i64)))"#,
    ] {
        assert!(super::inspect(&module("1 1", import, "i64.const 0"), &profile).is_err());
    }
}

#[test]
fn denies_wrong_run_abi_post_mvp_and_promotion() {
    let mut profile = super::super::profile::Profile::default();
    let wrong = wat::parse_str(
        r#"(module (memory (export "memory") 1 1)
        (func (export "molten_alloc") (param i32) (result i32) i32.const 0)
        (func (export "molten_dealloc") (param i32 i32))
        (func (export "molten_run") (param i32 i32) (result i32) i32.const 0))"#,
    )
    .expect("WAT");
    assert!(super::inspect(&wrong, &profile).is_err());
    let proposal = module("1 1", "", "i32.const 0 i32.extend8_s drop i64.const 0");
    assert!(super::inspect(&proposal, &profile).is_err());
    profile.production = true;
    assert!(super::inspect(&module("1 1", "", "i64.const 0"), &profile).is_err());
}
