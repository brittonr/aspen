#[path = "../cli/node.rs"]
mod node;

#[derive(clap::Parser)]
#[command(name = "molten-node", version, about = "Molten normal-node runtime")]
struct Cli {
    #[command(subcommand)]
    command: node::Command,
}

fn main() {
    let cli = <Cli as clap::Parser>::parse();
    if let Err(error) = node::run(cli.command) {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
