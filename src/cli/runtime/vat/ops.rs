pub(super) fn run(command: super::command::Top) -> molten::error::Result<()> {
    match command {
        super::command::Top::RunFixture { out } => {
            let run = molten::runtime::run_vat_fixture()?;
            write_fixture(&out, &run.value, &run.run_ref, "vat fixture run")
        }
        super::command::Top::SnapshotFixture { out } => {
            let snapshot = molten::runtime::run_vat_snapshot_fixture()?;
            write_fixture(&out, &snapshot.value, &snapshot.fixture_ref, "vat snapshot fixture")
        }
        super::command::Top::RestoreFixture { out } => {
            let restore = molten::runtime::run_vat_restore_fixture()?;
            write_fixture(&out, &restore.value, &restore.fixture_ref, "vat restore fixture")
        }
        super::command::Top::PromiseFixture { out } => {
            let promise = molten::runtime::run_vat_promise_fixture()?;
            write_fixture(&out, &promise.value, &promise.fixture_ref, "vat promise fixture")
        }
        super::command::Top::AmbientAuthorityFixture { out } => {
            let authority = molten::runtime::run_vat_ambient_authority_fixture()?;
            write_fixture(&out, &authority.value, &authority.fixture_ref, "vat ambient authority fixture")
        }
        super::command::Top::RightsFixture { out } => {
            let rights = molten::runtime::run_vat_rights_fixture()?;
            write_fixture(&out, &rights.value, &rights.fixture_ref, "vat rights fixture")
        }
        super::command::Top::DistributedRefFixture { out } => {
            let distributed_ref = molten::runtime::run_vat_distributed_ref_fixture()?;
            write_fixture(&out, &distributed_ref.value, &distributed_ref.fixture_ref, "vat distributed ref fixture")
        }
        super::command::Top::TimeTravelFixture { out } => {
            let debug = molten::runtime::run_vat_time_travel_fixture()?;
            write_fixture(&out, &debug.value, &debug.fixture_ref, "vat time travel fixture")
        }
        super::command::Top::ReplayFixture { out } => {
            let replay = molten::runtime::run_vat_replay_fixture()?;
            write_fixture(&out, &replay.value, &replay.fixture_ref, "vat replay fixture")
        }
        super::command::Top::AuthorityGraphFixture { out } => {
            let graph = molten::runtime::run_vat_authority_graph_fixture()?;
            write_fixture(&out, &graph.value, &graph.fixture_ref, "vat authority graph fixture")
        }
        super::command::Top::PortableStorageFixture { out } => {
            let storage = molten::runtime::run_vat_portable_storage_fixture()?;
            write_fixture(&out, &storage.value, &storage.fixture_ref, "vat portable storage fixture")
        }
        super::command::Top::Show { report } => {
            let value = super::io::read_preserves_file(&report)?;
            println!("{}", molten::runtime::vat_fixture_summary(&value)?);
            Ok(())
        }
    }
}

fn write_fixture(
    out: &std::path::Path,
    value: &preserves::IOValue,
    reference: &str,
    label: &str,
) -> molten::error::Result<()> {
    super::io::write_file(out, &molten::preserves_rail::to_text(value)?)?;
    println!("{label}: {reference}");
    Ok(())
}
