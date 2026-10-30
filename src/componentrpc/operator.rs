//! Capability-rooted operator surface. Report publication is last; no ambient paths.
use std::io::Write;

use cap_fs_ext::OpenOptionsFollowExt;

pub(crate) fn persist(root: &cap_std::fs::Dir, leaf: &str, value: &preserves::IOValue) -> super::Result<()> {
    let bytes = crate::codec::canonical_bytes(value).map_err(super::failure)?;
    let mut options = cap_std::fs::OpenOptions::new();
    options.write(true).create_new(true).follow(cap_fs_ext::FollowSymlinks::No);
    let mut file = root.open_with(leaf, &options).map_err(super::failure)?;
    file.write_all(&bytes).map_err(super::failure)?;
    file.sync_all().map_err(super::failure)
}
fn recorded(root: &cap_std::fs::Dir, leaf: &str) -> super::Result<super::transcript::Transcript> {
    let bytes = super::package::read(root, leaf)?;
    let value = crate::codec::parse_canonical_bytes(&bytes).map_err(super::failure)?;
    super::transcript::Transcript::from_value(&value)
}

pub async fn run(
    package: &cap_std::fs::Dir,
    staging: &cap_std::fs::Dir,
    output: &cap_std::fs::Dir,
) -> super::Result<preserves::IOValue> {
    let bundle = super::materialization::Bundle::load_package(package, staging)?;
    let mut result = super::fixture::run(&bundle, output).await?;
    result.negative_cases.extend(super::package::negative_cases(package)?);
    let mut fields = vec![
        crate::codec::string(super::PROFILE),
        crate::codec::string(bundle.reference()),
        crate::codec::string(&result.client.reference),
        crate::codec::string(&result.server.reference),
        crate::codec::string(result.client_result.to_string()),
        crate::codec::string(&result.replay_result),
    ];
    fields.push(crate::codec::record(
        "negative-cases",
        result.negative_cases.iter().map(crate::codec::string).collect(),
    ));
    fields.push(crate::codec::record(
        "non-claims",
        super::profile::NONCLAIMS.iter().map(crate::codec::string).collect(),
    ));
    let report = crate::codec::record("component-rpc-pilot-report-v1", fields);
    persist(output, "client.preserves", &result.client.value()?)?;
    persist(output, "server.preserves", &result.server.value()?)?;
    persist(output, "report.preserves", &report)?;
    Ok(report)
}

pub fn show(output: &cap_std::fs::Dir) -> super::Result<preserves::IOValue> {
    let bytes = super::package::read(output, "report.preserves")?;
    let value = crate::codec::parse_canonical_bytes(&bytes).map_err(super::failure)?;
    let fields =
        crate::codec::simple_record_fields(&value, "component-rpc-pilot-report-v1", 8).map_err(super::failure)?;
    let client = recorded(output, "client.preserves")?;
    let server = recorded(output, "server.preserves")?;
    let client_ref = crate::codec::required_string_field(&fields[2], "client transcript").map_err(super::failure)?;
    let server_ref = crate::codec::required_string_field(&fields[3], "server transcript").map_err(super::failure)?;
    if client.reference != client_ref || server.reference != server_ref || client.result_wire != server.result_wire {
        return Err("readback report does not bind both canonical transcripts".into());
    }
    Ok(value)
}

pub fn replay(output: &cap_std::fs::Dir, bundle: &super::materialization::Bundle) -> super::Result<String> {
    show(output)?;
    let transcript = recorded(output, "client.preserves")?;
    bundle.check(&transcript.scope)?;
    super::transcript::replay(&transcript.scope, &transcript)
}
