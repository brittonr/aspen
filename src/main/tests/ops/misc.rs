    include!("misc/coordination.rs");

    #[test]
    fn cli_secrets_commands_work() {
        let dir = temp_dir("secrets-cli");
        let out = dir.join("secrets-fixture");
        run_secrets_command(SecretsCommand::RunFixture { out: out.clone() }).expect("secrets fixture");
        let report = out.join("report.preserves");
        let report_value = read_preserves_file(&report).expect("read secrets report");
        let summary = secrets::fixture_report_summary(&report_value).expect("summary");
        assert!(summary.contains("plaintext=redacted"));
        let secret = out.join("secret.preserves");
        let secret_value = read_preserves_file(&secret).expect("read secret");
        let parsed = secrets::parse_secret_ref(&secret_value).expect("parse secret");
        assert_eq!(parsed.secret_id, "secret:fixture");
        run_secrets_command(SecretsCommand::Show { artifact: report }).expect("show report");
        run_secrets_command(SecretsCommand::Show { artifact: secret }).expect("show secret");
    }

    #[test]
    fn cli_plugin_lifecycle_commands_work() {
        let dir = temp_dir("plugin-cli");
        let state_root = dir.join("state");
        let out = dir.join("plugin-fixture");
        run_plugin_command(PluginCommand::RunFixture {
            state_root,
            out: out.clone(),
        })
        .expect("plugin fixture");
        let report = out.join("report.preserves");
        let report_value = read_preserves_file(&report).expect("read plugin report");
        assert!(to_text(&report_value).expect("render plugin report").contains("plugin-fixture-report-v1"));
        let manifest = out.join("evidence-0.preserves");
        let manifest_value = read_preserves_file(&manifest).expect("read plugin manifest");
        let parsed = plugin_host::parse_plugin_manifest(&manifest_value).expect("parse plugin manifest");
        assert_eq!(parsed.plugin_id, "plugin:minimal");
        run_plugin_command(PluginCommand::Show { artifact: report }).expect("plugin show report");
        run_plugin_command(PluginCommand::Show { artifact: manifest }).expect("plugin show manifest");
    }

    include!("misc/schema.rs");
    include!("misc/upgrade.rs");
