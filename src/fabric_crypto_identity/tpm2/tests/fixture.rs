use super::*;

pub(super) struct Provisioned {
    pub(super) scope: crate::node_state::Scope,
    pub(super) executable: crate::node_state::Executable,
    pub(super) workspace: crate::node_state::Workspace,
    pub(super) record: Vec<u8>,
    pub(super) expected_secret: iroh::SecretKey,
    pub(super) profile: Profile,
    pub(super) policy: Tpm2SealedPolicy,
}

impl Provisioned {
    pub(super) fn new() -> Self {
        let root = std::env::var_os("MOLTEN_TPM2_FIXTURE_ROOT").expect("explicit disposable fixture root");
        let tool = std::path::PathBuf::from(std::env::var_os("MOLTEN_TPM2_TOOL").expect("pinned fixture tool"));
        let tool_root =
            crate::node_state::DirectoryRoot::open_existing(tool.parent().expect("tool parent")).expect("tool root");
        let tool_leaf = crate::node_state::RelativePath::parse(
            tool.file_name().expect("tool leaf").to_str().expect("UTF8 tool leaf"),
        )
        .expect("relative tool");
        let executable = tool_root.open_executable(&tool_leaf).expect("retained tool");
        let scope = crate::node_state::Scope::open_existing(
            crate::node_state::NamespaceKind::Identity,
            std::path::Path::new(&root),
        )
        .expect("fixture namespace");
        let workspace = scope.process_workspace().expect("retained fixture workspace");
        fixture_tool(
            &executable,
            &workspace,
            &["createprimary", "-C", "o", "-c", "primary.ctx", "-Q"],
            bounded_exec::Input::Null,
        );
        fixture_tool(
            &executable,
            &workspace,
            &[
                "createpolicy",
                "--policy-pcr",
                "-l",
                "sha256:16",
                "-L",
                "pcr.policy",
                "-Q",
            ],
            bounded_exec::Input::Null,
        );
        let mut record = super::super::super::file_adapter::generate_transport_key_record();
        record[8..16].copy_from_slice(&2u64.to_be_bytes());
        let (_, expected_secret) =
            super::super::super::file_adapter::decode_transport_record(&record).expect("memory-only generated record");
        load_object(&executable, &workspace, &record);
        let mut policy = sample_policy();
        policy.public_object = scope
            .read(&crate::node_state::RelativePath::parse("sealed.pub").expect("public leaf"), 1_024)
            .expect("actual public object");
        policy.executable_ref = crate::preserves_rail::content_ref_from_bytes(
            &executable.read_bounded(crate::node_state::MAX_EXECUTABLE_BYTES).expect("actual tool bytes"),
        );
        let profile = super::super::super::profile(&production_ed25519_profile(
            reference("production-profile"),
            reference("os-csprng"),
        ))
        .expect("production profile");
        Self {
            scope,
            executable,
            workspace,
            record,
            expected_secret,
            profile,
            policy,
        }
    }
}

fn load_object(executable: &crate::node_state::Executable, workspace: &crate::node_state::Workspace, record: &[u8]) {
    fixture_tool(
        executable,
        workspace,
        &[
            "create",
            "-C",
            "primary.ctx",
            "-i",
            "-",
            "-L",
            "pcr.policy",
            "-a",
            "fixedtpm|fixedparent",
            "-u",
            "sealed.pub",
            "-r",
            "sealed.priv",
            "-Q",
        ],
        bounded_exec::Input::Bytes(record.to_vec()),
    );
    fixture_tool(executable, workspace, &["flushcontext", "-t"], bounded_exec::Input::Null);
    fixture_tool(
        executable,
        workspace,
        &[
            "load",
            "-C",
            "primary.ctx",
            "-u",
            "sealed.pub",
            "-r",
            "sealed.priv",
            "-c",
            "sealed.ctx",
            "-Q",
        ],
        bounded_exec::Input::Null,
    );
    fixture_tool(
        executable,
        workspace,
        &["evictcontrol", "-C", "o", "-c", "sealed.ctx", "0x81000010", "-Q"],
        bounded_exec::Input::Null,
    );
    fixture_tool(executable, workspace, &["flushcontext", "-t"], bounded_exec::Input::Null);
}
