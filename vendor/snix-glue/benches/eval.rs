use clap::Parser;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use mimalloc::MiMalloc;
use snix_build::buildservice::DummyBuildService;
use snix_eval::{EvalIO, builtins::impure_builtins};
use snix_glue::{
    builtins::{add_derivation_builtins, add_fetcher_builtins, add_import_builtins},
    configure_nix_path,
    snix_io::SnixIO,
    snix_store_io::SnixStoreIO,
};
use snix_store::utils::{ServiceUrlsMemory, construct_services};
use std::sync::LazyLock;
use std::{env, rc::Rc, sync::Arc, time::Duration};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

static TOKIO_RUNTIME: LazyLock<tokio::runtime::Runtime> =
    LazyLock::new(|| tokio::runtime::Runtime::new().unwrap());

fn interpret(code: &str) {
    // TODO: this is a bit annoying.
    // It'd be nice if we could set this up once and then run evaluate() with a
    // piece of code. b/262
    let (blob_service, directory_service, path_info_service, nar_calculation_service) =
        TOKIO_RUNTIME
            .block_on(async {
                construct_services(ServiceUrlsMemory::parse_from(std::iter::empty::<&str>())).await
            })
            .unwrap();

    // We assemble a complete store in memory.
    let snix_store_io = Rc::new(SnixStoreIO::new(
        blob_service,
        directory_service,
        path_info_service,
        nar_calculation_service.into(),
        Arc::<DummyBuildService>::default(),
        TOKIO_RUNTIME.handle().clone(),
        Vec::new(),
    ));

    let mut eval_builder = snix_eval::Evaluation::builder(Box::new(SnixIO::new(
        snix_store_io.clone() as Rc<dyn EvalIO>,
    )) as Box<dyn EvalIO>)
    .enable_import()
    .add_builtins(impure_builtins());

    eval_builder = add_derivation_builtins(eval_builder, Rc::clone(&snix_store_io));
    eval_builder = add_fetcher_builtins(eval_builder, Rc::clone(&snix_store_io));
    eval_builder = add_import_builtins(eval_builder, snix_store_io);
    eval_builder = configure_nix_path(
        eval_builder,
        // The benchmark requires SNIX_BENCH_NIX_PATH to be set, so barf out
        // early, rather than benchmarking snix returning an error.
        &Some(env::var("SNIX_BENCH_NIX_PATH").expect("SNIX_BENCH_NIX_PATH must be set")),
    );

    let eval = eval_builder.build();
    let result = eval.evaluate(code, None);

    assert!(result.errors.is_empty(), "{:#?}", result.errors);
}

fn eval_nixpkgs(c: &mut Criterion) {
    c.bench_function("hello outpath", |b| {
        b.iter(|| {
            interpret(black_box("(import <nixpkgs> {}).hello.outPath"));
        })
    });

    c.bench_function("firefox outpath", |b| {
        b.iter(|| {
            interpret(black_box("(import <nixpkgs> {}).firefox.outPath"));
        })
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(30)).sample_size(10);
    targets = eval_nixpkgs
);
criterion_main!(benches);
