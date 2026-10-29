pub(super) const ECHO: &str = "i32.const 2048 i32.const 0 i32.store8
    i32.const 2052 local.get 0 i32.store
    i32.const 2056 local.get 1 i32.store i32.const 2048";

pub(super) fn encode(
    memory_pages: u32,
    core_items: &str,
    invoke_body: &str,
    other_components: &str,
    lift_options: &str,
) -> Vec<u8> {
    wat::parse_str(format!(
        r#"(component
  (core module $guest
    (memory (export "memory") {memory_pages} {memory_pages})
    (func (export "realloc") (param i32 i32 i32 i32) (result i32) i32.const 4096)
    (func (export "invoke") (param i32 i32) (result i32) {invoke_body})
    {core_items})
  (core instance $i (instantiate $guest))
  {other_components}
  (func (export "invoke")
    (param "input" (list u8))
    (result (result (list u8) (error string)))
    (canon lift (core func $i "invoke")
      (memory $i "memory") (realloc (func $i "realloc")) {lift_options})))"#
    ))
    .expect("actual parity component")
}

pub(super) fn start_trap() -> Vec<u8> {
    encode(1, "(func $start unreachable) (start $start)", ECHO, "", "")
}

pub(super) fn invoke_trap() -> Vec<u8> {
    encode(1, "", "unreachable", "", "")
}

pub(super) fn guest_denial() -> Vec<u8> {
    encode(1, "", "i32.const 2048 i32.const 1 i32.store8 i32.const 2048", "", "")
}

pub(super) fn failed_growth() -> Vec<u8> {
    encode(1, "", &format!("i32.const 1 memory.grow drop {ECHO}"), "", "")
}

pub(super) fn cleanup() -> Vec<u8> {
    encode(
        1,
        "(func (export \"post\") (param i32) i32.const 0 i32.const 7 i32.store)",
        ECHO,
        "",
        "(post-return (func $i \"post\"))",
    )
}

pub(super) fn cleanup_trap() -> Vec<u8> {
    encode(1, "(func (export \"post\") (param i32) unreachable)", ECHO, "", "(post-return (func $i \"post\"))")
}

pub(super) fn cleanup_exhaustion() -> Vec<u8> {
    encode(
        1,
        "(func (export \"post\") (param i32) (loop $forever (br $forever)))",
        ECHO,
        "",
        "(post-return (func $i \"post\"))",
    )
}

pub(super) fn undeclared_import() -> Vec<u8> {
    encode(
        1,
        "",
        ECHO,
        "(type $ambient (instance (export \"read\" (func))))
         (import \"wasi:cli/environment@0.2.6\" (instance (type $ambient)))",
        "",
    )
}

pub(super) fn expensive_start() -> Vec<u8> {
    encode(
        1,
        r#"(func $start (local $remaining i32)
    i32.const 3000 local.set $remaining
    (loop $work
      local.get $remaining i32.const 1 i32.sub local.tee $remaining br_if $work))
    (start $start)"#,
        ECHO,
        "",
        "",
    )
}

pub(super) fn aggregate_memory() -> Vec<u8> {
    encode(
        129,
        "",
        ECHO,
        r#"
          (core module $other (memory 129 129))
          (core instance $other-instance (instantiate $other))"#,
        "",
    )
}

pub(super) fn oversized_invalid_span() -> Vec<u8> {
    encode(
        1,
        "",
        "i32.const 2048 i32.const 0 i32.store8
    i32.const 2052 i32.const 0 i32.store
    i32.const 2056 i32.const 65537 i32.store i32.const 2048",
        "",
        "",
    )
}

pub(super) fn oversized_valid_span() -> Vec<u8> {
    encode(
        2,
        "",
        "i32.const 2048 i32.const 0 i32.store8
    i32.const 2052 i32.const 0 i32.store
    i32.const 2056 i32.const 65537 i32.store i32.const 2048",
        "",
        "",
    )
}
