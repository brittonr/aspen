// SPDX-License-Identifier: AGPL-3.0-or-later
use std::path::PathBuf;

use anyhow::Result;
use clap::Subcommand;
use clap::ValueEnum;
use molten_orchestrator_adapters as adapters;
use molten_pure_net as net;

#[derive(Clone, Copy, ValueEnum)]
pub enum Backend {
    SequentialNet,
    IndependentReference,
}

#[derive(Subcommand)]
pub enum Command {
    /// Freeze this actual executable/compiled Rust closure and finite limits.
    Manifest {
        #[arg(long, value_enum)]
        backend: Backend,
        #[arg(long)]
        out: PathBuf,
        #[arg(
            long,
            help = "Generated JSON limits, for example exported from the reviewed Nickel profile"
        )]
        limits: Option<PathBuf>,
    },
    /// Check source with the pinned producer and materialize canonical inputs.
    Compile {
        #[arg(long)]
        source: PathBuf,
        #[arg(long)]
        input_arity: u32,
        #[arg(long)]
        inputs: PathBuf,
        #[arg(long)]
        program_out: PathBuf,
        #[arg(long)]
        inputs_out: PathBuf,
    },
    /// Explicit create-only installation into an initialized native root.
    Install {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        manifest: PathBuf,
    },
    /// Verify/reconstruct a canonical semantic receipt; never an authority grant.
    Receipt {
        #[arg(long)]
        file: PathBuf,
    },
}

pub fn run(command: Command) -> Result<()> {
    match command {
        Command::Manifest { backend, out, limits } => {
            let limits = match limits {
                Some(path) => serde_json::from_slice(&super::read_file(&path, 32768)?)?,
                None => net::Limits::default(),
            };
            let backend = match backend {
                Backend::SequentialNet => net::Backend::SequentialNet,
                Backend::IndependentReference => net::Backend::IndependentReference,
            };
            let manifest = net::manifest(adapters::pure_net_host_bindings()?, backend, limits)?;
            let bytes = net::encode_manifest(&manifest)?;
            let id = net::content_id_manifest(&bytes)?;
            super::write_new(&out, &bytes)?;
            super::emit(serde_json::json!({"manifest":id,"profile":manifest,"native_trusted_not_sandboxed":true}))?;
        }
        Command::Compile {
            source,
            input_arity,
            inputs,
            program_out,
            inputs_out,
        } => {
            let source = super::read_file(&source, net::Limits::default().max_source_bytes as usize)?;
            let source = std::str::from_utf8(&source)?;
            let values: Vec<i64> = serde_json::from_slice(&super::read_file(&inputs, 32768)?)?;
            let program = net::compile(source, input_arity)?;
            let input = net::encode_inputs(&values)?;
            let program_id = net::content_id_program(&program)?;
            let input_id = net::content_id_inputs(&input)?;
            super::write_new(&program_out, &program)?;
            super::write_new(&inputs_out, &input)?;
            super::emit(serde_json::json!({"program":program_id,"input":input_id,"producer":net::PRODUCER_REVISION}))?;
        }
        Command::Install { root, manifest } => {
            let bytes = super::read_file(&manifest, 32768)?;
            let id = adapters::install_pure_net(&super::root(&root)?, bytes)?;
            super::emit(serde_json::json!({"manifest":id,"installed":true,"scope":"standalone-root"}))?;
        }
        Command::Receipt { file } => {
            let receipt = net::decode_receipt(&super::read_file(&file, molten_batch_core::MAX_CONTENT_BYTES)?)?;
            super::emit(serde_json::to_value(receipt)?)?;
        }
    }
    Ok(())
}
