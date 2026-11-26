use super::*;

const OPERATION: &str = "assert";

fn prepared(source: &str, callable: &str) -> std::result::Result<Prepared, String> {
    prepare_run("fixture", source, callable, &crate::preserves_rail::string("service.ready"))
        .map_err(|error| error.to_string())
}

fn run(source: &str) -> SteelRun {
    let prepared = match prepared(source, "main") {
        Ok(prepared) => prepared,
        Err(error) => panic!("reviewed Steel source must be admitted: {error}"),
    };
    run_vm(&prepared, OPERATION, &[OPERATION.to_string()])
}

fn rejection(source: &str, callable: &str) -> String {
    match prepared(source, callable) {
        Ok(prepared) => panic!("Steel source must be rejected before execution: {}", prepared.program),
        Err(error) => error,
    }
}

// r[verify molten.bounded_execution.steel_sandbox_and_real_bounds]
#[test]
fn legitimate_script_runs_with_real_metering_and_deterministic_receipt_inputs() {
    let source = r#"
        (define (shout text) (string-append text "!"))
        (define (main input)
          (if (string=? (molten-hostcall "assert" input) "")
              "unreachable"
              (shout input)))
    "#;
    let first = run(source);
    let second = run(source);
    assert_eq!(first.meter, second.meter, "metering must be deterministic");
    assert_eq!(first.outcome, second.outcome);
    let output = first.outcome.clone().expect("legitimate script completes");
    assert_eq!(output, r#""\"service.ready\"!""#, "the input is the actor's Preserves text, passed as one string");
    assert_eq!(first.meter.exhaustion, None);
    assert_eq!(first.meter.hostcall_count, 1);
    assert!(first.meter.fuel_remaining < STEEL_FUEL_LIMIT, "lambda entries and primitives must spend fuel");
    assert!(first.meter.allocated_bytes > 0, "the input and produced strings must be charged");
    let execution = first.into_result("fixture").expect("bounded run is admitted");
    assert_eq!(execution.hostcall_count, 1);
}

// r[verify molten.bounded_execution.steel_sandbox_and_real_bounds]
#[test]
fn ambient_filesystem_process_and_environment_access_is_unavailable() {
    for source in [
        r#"(define (main input) (open-input-file "/etc/passwd"))"#,
        r#"(define (main input) (read-port-to-string (open-input-file "/etc/passwd")))"#,
        r#"(define (main input) (delete-file! "/tmp/molten"))"#,
        r#"(require "ambient.scm") (define (main input) input)"#,
        r#"(define (main input) (require-builtin steel/process))"#,
        r#"(define (main input) (command "sh" (list "-c" "id")))"#,
        r#"(define (main input) (env-var "HOME"))"#,
        r#"(define (main input) std::env::args)"#,
        r#"(define (main input) (displayln input))"#,
        r#"(define (main input) (current-milliseconds))"#,
        r#"(define (main input) (spawn-thread! (lambda () input)))"#,
        r#"(define (main input) (#%module "steel/process"))"#,
        r#"(define (main input) (load-from-module! input "command"))"#,
        r#"(define (main input) (make-string 9000 #\a))"#,
    ] {
        let error = rejection(source, "main");
        assert!(error.contains("outside the reviewed Steel sandbox"), "{source}: {error}");
    }
}

// r[verify molten.bounded_execution.steel_sandbox_and_real_bounds]
#[test]
fn macros_mutation_and_non_definition_forms_are_rejected() {
    for source in [
        "(define-syntax loop (syntax-rules () ((_ x) (loop x)))) (define (main input) input)",
        "(define (main input) (begin (set! input 1) input))",
        "(define (main input) (define inner 1) input)",
        "(define (main input) (if input input))",
        "(define (main . inputs) inputs)",
        "(main \"x\")",
        "(define main 1) (define main 2)",
    ] {
        let error = rejection(source, "main");
        assert!(error.contains("outside the reviewed Steel sandbox"), "{source}: {error}");
    }
}

// r[verify molten.bounded_execution.steel_sandbox_and_real_bounds]
#[test]
fn shadowing_a_keyword_cannot_reach_the_compiler_namespace() {
    let source = r#"(define (main require-builtin) (require-builtin "steel/process"))"#;
    match prepared(source, "main") {
        Err(error) => assert!(error.contains("outside the reviewed Steel sandbox"), "{error}"),
        Ok(prepared) => {
            assert!(!prepared.program.contains("(require-builtin"), "{}", prepared.program);
            let run = run_vm(&prepared, OPERATION, &[OPERATION.to_string()]);
            assert!(run.outcome.is_err(), "a user string is not a procedure");
        }
    }
}

// r[verify molten.bounded_execution.steel_sandbox_and_real_bounds]
#[test]
fn anonymous_lambda_recursion_exhausts_real_fuel_deterministically() {
    for source in [
        "(define (main input) ((lambda (x) (x x)) (lambda (x) (x x))))",
        "(define (main input) (main input))",
    ] {
        let first = run(source);
        let second = run(source);
        assert_eq!(first.meter.exhaustion, Some(Exhaustion::Fuel), "{source}");
        assert_eq!(first.meter.fuel_remaining, 0);
        assert!(first.outcome.is_err());
        assert_eq!(first.meter, second.meter, "{source}: fuel exhaustion must be deterministic");
        let error = first.into_result("fixture").err().expect("fuel exhaustion denies").to_string();
        assert!(error.contains("exhausted its deterministic fuel limit"), "{error}");
    }
}

// r[verify molten.bounded_execution.steel_sandbox_and_real_bounds]
#[test]
fn string_doubling_exhausts_the_memory_ceiling_before_fuel() {
    let doubling = run("(define (grow text) (grow (string-append text text))) (define (main input) (grow input))");
    assert_eq!(doubling.meter.exhaustion, Some(Exhaustion::Memory));
    assert!(doubling.meter.fuel_remaining > 0, "memory, not fuel, must stop doubling");

    let cumulative = run(r#"
        (define (grow text count) (if (= count 0) text (grow (string-append text text) (- count 1))))
        (define (burn text count) (if (= count 0) "done" (begin (string-append text text) (burn text (- count 1)))))
        (define (main input) (burn (grow input 8) 64))
    "#);
    assert_eq!(cumulative.meter.exhaustion, Some(Exhaustion::Memory));
    assert!(cumulative.meter.allocated_bytes <= STEEL_MEMORY_LIMIT_BYTES);
    let error = cumulative.into_result("fixture").err().expect("memory exhaustion denies").to_string();
    assert!(error.contains("exhausted its deterministic memory limit"), "{error}");
}

// r[verify molten.bounded_execution.steel_sandbox_and_real_bounds]
#[test]
fn hostcall_flood_is_refused_before_the_excess_call_dispatches() {
    let script = |count: u64| {
        format!(
            r#"(define (calls n) (if (= n 0) "done" (begin (molten-hostcall "assert" "envelope") (calls (- n 1)))))
               (define (main input) (calls {count}))"#
        )
    };
    let bounded = run(&script(STEEL_MAX_HOSTCALLS));
    assert_eq!(bounded.meter.exhaustion, None);
    assert_eq!(bounded.meter.hostcall_count, STEEL_MAX_HOSTCALLS);
    assert!(bounded.outcome.is_ok());

    let flood = run(&script(STEEL_MAX_HOSTCALLS * 4));
    assert_eq!(flood.meter.exhaustion, Some(Exhaustion::Hostcalls));
    assert_eq!(flood.meter.hostcall_count, STEEL_MAX_HOSTCALLS, "the excess hostcall must never dispatch");
    assert!(flood.outcome.is_err());
}

// r[verify molten.bounded_execution.steel_sandbox_and_real_bounds]
#[test]
fn callable_injection_is_rejected_without_interpolation() {
    let source = "(define (main input) input)";
    for callable in [
        "main) (open-input-file \"/etc/passwd\"",
        "main \"x\")",
        "(main",
        "main;comment",
        "main\n(displayln 1)",
        "",
        "1main",
    ] {
        let error = rejection(source, callable);
        assert!(error.contains("callable"), "{callable:?}: {error}");
    }
    let error = rejection(source, "string-append");
    assert!(error.contains("is not a top-level definition"), "{error}");

    let prepared = prepared(source, "main").expect("closed callable is admitted");
    assert!(!prepared.program.contains("service.ready"), "input is passed as a value, never as source text");
    assert!(run(source).outcome.is_ok());
}

// r[verify molten.bounded_execution.steel_sandbox_and_real_bounds]
#[test]
fn closure_chain_is_bounded_and_dropped_off_the_callers_stack() {
    let run = std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| run("(define (grow k) (grow (lambda () k))) (define (main input) (grow input))"))
        .expect("spawn 2 MiB caller")
        .join()
        .expect("a closure chain must not overflow the caller's stack");
    assert_eq!(run.meter.exhaustion, Some(Exhaustion::Memory), "closure creation is charged");
    assert!(run.meter.allocated_bytes <= STEEL_MEMORY_LIMIT_BYTES);
}

// r[verify molten.bounded_execution.steel_sandbox_and_real_bounds]
#[test]
fn closure_creation_counts_against_the_memory_ceiling() {
    let run = run(r#"
        (define (spin n) (if (= n 0) "done" (begin (lambda (a b c) n) (lambda () n) (spin (- n 1)))))
        (define (main input) (spin 100000))
    "#);
    assert_eq!(run.meter.exhaustion, Some(Exhaustion::Memory));
    assert!(run.meter.fuel_remaining > 0, "memory, not fuel, must stop closure creation");

    let wide = (0..=STEEL_MAX_ARITY).map(|index| format!("p{index}")).collect::<Vec<_>>().join(" ");
    let error = rejection(&format!("(define (main {wide}) input)"), "main");
    assert!(error.contains("at most"), "{error}");
    let arguments = vec!["1"; STEEL_MAX_ARITY + 1].join(" ");
    let error = rejection(&format!("(define (main input) (+ {arguments}))"), "main");
    assert!(error.contains("at most"), "{error}");
}

// r[verify molten.bounded_execution.steel_sandbox_and_real_bounds]
#[test]
fn arithmetic_on_locals_reaches_the_metered_checked_primitives() {
    let counted = run(r#"
        (define (count n) (if (= n 0) "done" (count (- n 1))))
        (define (main input) (count 100))
    "#);
    assert_eq!(counted.meter.exhaustion, None);
    // Two closures at load, one entry for main, 101 entries, 101 comparisons, and 100 subtractions.
    assert_eq!(STEEL_FUEL_LIMIT - counted.meter.fuel_remaining, 2 + 1 + 101 + 101 + 100);

    for source in [
        "(define (bump n) (+ n 1)) (define (main input) (bump 9223372036854775807))",
        "(define (drop n) (- n 1)) (define (main input) (drop -9223372036854775808))",
        "(define (main input) (+ 9223372036854775807 1))",
    ] {
        let overflow = run(source);
        let error = overflow.outcome.expect_err("checked arithmetic must refuse overflow");
        assert!(error.contains("overflowed"), "{source}: {error}");
    }
    assert!(!prepared("(define (main input) (+ input 1))", "main").expect("admitted").program.contains("(+ "));
}

// r[verify molten.bounded_execution.steel_sandbox_and_real_bounds]
#[test]
fn deeply_nested_source_is_rejected_without_using_the_callers_stack() {
    // About 7.8 KB: under the source byte bound, so only the nesting check can reject it.
    let nested = format!("(define (main input) {}input{})", "(".repeat(3_900), ")".repeat(3_900));
    assert!(nested.len() <= STEEL_MAX_SOURCE_BYTES);
    let error = std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || prepared(&nested, "main").err())
        .expect("spawn 2 MiB caller")
        .join()
        .expect("deep nesting must not overflow the caller's stack")
        .expect("deep nesting is rejected");
    assert!(error.contains("brackets nest deeper than"), "{error}");

    let quoted = format!(r#"(define (main input) (string-append input "{}"))"#, "(".repeat(500));
    assert!(run(&quoted).outcome.is_ok(), "brackets inside string literals do not count");
}
