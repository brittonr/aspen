use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::Subcommand;
use molten::error::MoltenError;
use molten::error::Result;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::to_text;
use molten::runtime;

#[derive(Debug, Subcommand)]
pub(crate) enum VatCommand {
    RunFixture {
        #[arg(long)]
        out: PathBuf,
    },
    SnapshotFixture {
        #[arg(long)]
        out: PathBuf,
    },
    RestoreFixture {
        #[arg(long)]
        out: PathBuf,
    },
    PromiseFixture {
        #[arg(long)]
        out: PathBuf,
    },
    AmbientAuthorityFixture {
        #[arg(long)]
        out: PathBuf,
    },
    RightsFixture {
        #[arg(long)]
        out: PathBuf,
    },
    DistributedRefFixture {
        #[arg(long)]
        out: PathBuf,
    },
    TimeTravelFixture {
        #[arg(long)]
        out: PathBuf,
    },
    ReplayFixture {
        #[arg(long)]
        out: PathBuf,
    },
    AuthorityGraphFixture {
        #[arg(long)]
        out: PathBuf,
    },
    PortableStorageFixture {
        #[arg(long)]
        out: PathBuf,
    },
    Show {
        report: PathBuf,
    },
}

pub(crate) fn run_vat_command(command: VatCommand) -> Result<()> {
    match command {
        VatCommand::RunFixture { out } => {
            let run = runtime::run_vat_fixture()?;
            write_file(&out, &to_text(&run.value)?)?;
            println!("vat fixture run: {}", run.run_ref);
            Ok(())
        }
        VatCommand::SnapshotFixture { out } => {
            let snapshot = runtime::run_vat_snapshot_fixture()?;
            write_file(&out, &to_text(&snapshot.value)?)?;
            println!("vat snapshot fixture: {}", snapshot.fixture_ref);
            Ok(())
        }
        VatCommand::RestoreFixture { out } => {
            let restore = runtime::run_vat_restore_fixture()?;
            write_file(&out, &to_text(&restore.value)?)?;
            println!("vat restore fixture: {}", restore.fixture_ref);
            Ok(())
        }
        VatCommand::PromiseFixture { out } => {
            let promise = runtime::run_vat_promise_fixture()?;
            write_file(&out, &to_text(&promise.value)?)?;
            println!("vat promise fixture: {}", promise.fixture_ref);
            Ok(())
        }
        VatCommand::AmbientAuthorityFixture { out } => {
            let authority = runtime::run_vat_ambient_authority_fixture()?;
            write_file(&out, &to_text(&authority.value)?)?;
            println!("vat ambient authority fixture: {}", authority.fixture_ref);
            Ok(())
        }
        VatCommand::RightsFixture { out } => {
            let rights = runtime::run_vat_rights_fixture()?;
            write_file(&out, &to_text(&rights.value)?)?;
            println!("vat rights fixture: {}", rights.fixture_ref);
            Ok(())
        }
        VatCommand::DistributedRefFixture { out } => {
            let distributed_ref = runtime::run_vat_distributed_ref_fixture()?;
            write_file(&out, &to_text(&distributed_ref.value)?)?;
            println!("vat distributed ref fixture: {}", distributed_ref.fixture_ref);
            Ok(())
        }
        VatCommand::TimeTravelFixture { out } => {
            let debug = runtime::run_vat_time_travel_fixture()?;
            write_file(&out, &to_text(&debug.value)?)?;
            println!("vat time travel fixture: {}", debug.fixture_ref);
            Ok(())
        }
        VatCommand::ReplayFixture { out } => {
            let replay = runtime::run_vat_replay_fixture()?;
            write_file(&out, &to_text(&replay.value)?)?;
            println!("vat replay fixture: {}", replay.fixture_ref);
            Ok(())
        }
        VatCommand::AuthorityGraphFixture { out } => {
            let graph = runtime::run_vat_authority_graph_fixture()?;
            write_file(&out, &to_text(&graph.value)?)?;
            println!("vat authority graph fixture: {}", graph.fixture_ref);
            Ok(())
        }
        VatCommand::PortableStorageFixture { out } => {
            let storage = runtime::run_vat_portable_storage_fixture()?;
            write_file(&out, &to_text(&storage.value)?)?;
            println!("vat portable storage fixture: {}", storage.fixture_ref);
            Ok(())
        }
        VatCommand::Show { report } => {
            let value = read_preserves_file(&report)?;
            println!("{}", runtime::vat_fixture_summary(&value)?);
            Ok(())
        }
    }
}

fn read_preserves_file(path: &Path) -> Result<preserves::IOValue> {
    let text = fs::read_to_string(path).map_err(MoltenError::from)?;
    parse_text(&text)
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    fs::write(path, contents).map_err(MoltenError::from)
}
