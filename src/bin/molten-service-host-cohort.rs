#[path = "servicehostcohort/mod.rs"]
mod authoring;

fn main() {
    if let Err(error) = authoring::run(<authoring::Cli as clap::Parser>::parse()) {
        eprintln!("service-host cohort authoring failed: {error}");
        std::process::exit(1);
    }
}
