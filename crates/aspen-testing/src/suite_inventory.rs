use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::bail;
use anyhow::ensure;
use nickel_lang::Context as NickelContext;
use nickel_lang::Error as NickelError;
use nickel_lang::ErrorFormat;
use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;
use walkdir::WalkDir;

const SCHEMA_VERSION: u32 = 1;
const GENERATED_STALE_MESSAGE: &str = "suite inventory is stale; run `scripts/test-harness.sh export`";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryPaths {
    pub repo_root: PathBuf,
    pub schema_path: PathBuf,
    pub manifest_root: PathBuf,
    pub output_path: PathBuf,
}

impl Default for InventoryPaths {
    fn default() -> Self {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        Self {
            schema_path: repo_root.join("test-harness/schema.ncl"),
            manifest_root: repo_root.join("test-harness/suites"),
            output_path: repo_root.join("test-harness/generated/inventory.json"),
            repo_root,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SuiteInventory {
    pub schema_version: u32,
    pub metadata: SuiteInventoryMetadata,
    pub suites: Vec<SuiteInventoryRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SuiteInventoryMetadata {
    pub schema_path: String,
    pub manifest_paths: Vec<String>,
    pub inputs_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SuiteInventoryRecord {
    pub id: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub layer: SuiteLayer,
    pub owner: String,
    pub runtime_class: RuntimeClass,
    pub prerequisites: Vec<Prerequisite>,
    pub tags: Vec<String>,
    pub manifest_path: String,
    pub target: SuiteTarget,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SuiteLayer {
    RustIntegration,
    Patchbay,
    Vm,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeClass {
    RealNetwork,
    LinuxNamespaces,
    NixosVm,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum Prerequisite {
    LinuxUserns,
    NetworkAccess,
    Nftables,
    NixCommand,
    TrafficControl,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SuiteTarget {
    pub kind: SuiteTargetKind,
    pub package: Option<String>,
    pub test: Option<String>,
    pub profile: Option<String>,
    pub features: Vec<String>,
    pub run_ignored: Option<RunIgnoredMode>,
    pub flake_attr: Option<String>,
    pub check_attr: Option<String>,
    pub nix_file: Option<String>,
    pub package_presets: BTreeMap<String, String>,
    pub register_flake_check: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SuiteTargetKind {
    CargoNextest,
    NixBuild,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RunIgnoredMode {
    Default,
    All,
    IgnoredOnly,
}

#[derive(Debug, Clone, Deserialize)]
struct SuiteManifest {
    id: String,
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    layer: String,
    owner: String,
    runtime_class: String,
    #[serde(default)]
    prerequisites: Vec<String>,
    #[serde(default)]
    tags: Vec<String>,
    target: RawSuiteTarget,
}

#[derive(Debug, Clone, Deserialize)]
struct RawSuiteTarget {
    kind: String,
    package: Option<String>,
    test: Option<String>,
    profile: Option<String>,
    #[serde(default)]
    features: Vec<String>,
    #[serde(default)]
    run_ignored: Option<String>,
    #[serde(default)]
    flake_attr: Option<String>,
    #[serde(default)]
    check_attr: Option<String>,
    #[serde(default)]
    nix_file: Option<String>,
    #[serde(default)]
    package_presets: BTreeMap<String, String>,
    #[serde(default)]
    register_flake_check: bool,
}

pub fn load_inventory(paths: &InventoryPaths) -> Result<SuiteInventory> {
    let schema_source = fs::read_to_string(&paths.schema_path)
        .with_context(|| format!("failed to read schema {}", paths.schema_path.display()))?;
    let manifest_paths = discover_manifest_paths(&paths.manifest_root)?;
    ensure!(!manifest_paths.is_empty(), "no suite manifests found under {}", paths.manifest_root.display());
    let metadata = build_inventory_metadata(paths, &manifest_paths)?;

    let mut seen_ids = BTreeSet::new();
    let mut seen_check_attrs = BTreeMap::new();
    let mut seen_flake_attrs = BTreeMap::new();
    let mut suites = Vec::with_capacity(manifest_paths.len());
    for manifest_path in manifest_paths {
        let manifest = load_manifest(&manifest_path, &schema_source)?;
        ensure!(
            seen_ids.insert(manifest.id.clone()),
            "duplicate suite id `{}` in {}",
            manifest.id,
            manifest_path.display()
        );
        validate_manifest(&manifest, &manifest_path, &paths.repo_root)?;
        let record = to_inventory_record(manifest, &manifest_path, &paths.repo_root)?;
        validate_generated_registration_keys(&record, &mut seen_check_attrs, &mut seen_flake_attrs)?;
        suites.push(record);
    }
    suites.sort_by(|left, right| left.id.cmp(&right.id));

    Ok(SuiteInventory {
        schema_version: SCHEMA_VERSION,
        metadata,
        suites,
    })
}

fn build_inventory_metadata(paths: &InventoryPaths, manifest_paths: &[PathBuf]) -> Result<SuiteInventoryMetadata> {
    let schema_path = repo_relative_path(&paths.schema_path, &paths.repo_root)?;
    let manifest_paths = manifest_paths
        .iter()
        .map(|path| repo_relative_path(path, &paths.repo_root))
        .collect::<Result<Vec<_>>>()?;

    let mut input_lines = Vec::with_capacity(manifest_paths.len() + 1);
    input_lines.push(format!("{schema_path}:{}", compute_file_sha256(&paths.schema_path)?));
    let mut manifest_entries = discover_manifest_hashes(paths, &manifest_paths)?;
    input_lines.append(&mut manifest_entries);

    let mut hasher = Sha256::new();
    hasher.update(input_lines.join("\n").as_bytes());

    Ok(SuiteInventoryMetadata {
        schema_path,
        manifest_paths,
        inputs_sha256: hex::encode(hasher.finalize()),
    })
}

fn discover_manifest_hashes(paths: &InventoryPaths, manifest_paths: &[String]) -> Result<Vec<String>> {
    manifest_paths
        .iter()
        .map(|relative_path| {
            let path = paths.repo_root.join(relative_path);
            Ok(format!("{relative_path}:{}", compute_file_sha256(&path)?))
        })
        .collect()
}

fn compute_file_sha256(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(hex::encode(hasher.finalize()))
}

pub fn render_inventory_json(inventory: &SuiteInventory) -> Result<String> {
    let mut json = serde_json::to_string_pretty(inventory).context("failed to serialize suite inventory")?;
    json.push('\n');
    Ok(json)
}

pub fn write_inventory(inventory: &SuiteInventory, output_path: &Path) -> Result<()> {
    let json = render_inventory_json(inventory)?;
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(output_path, json).with_context(|| format!("failed to write {}", output_path.display()))
}

pub fn ensure_inventory_is_current(inventory: &SuiteInventory, output_path: &Path) -> Result<()> {
    let expected = render_inventory_json(inventory)?;
    let actual = fs::read_to_string(output_path)
        .with_context(|| format!("failed to read generated inventory {}", output_path.display()))?;
    ensure!(actual == expected, "{GENERATED_STALE_MESSAGE}");
    Ok(())
}

fn discover_manifest_paths(manifest_root: &Path) -> Result<Vec<PathBuf>> {
    let mut manifest_paths = Vec::new();
    for entry in WalkDir::new(manifest_root) {
        let entry = entry.with_context(|| format!("failed to walk {}", manifest_root.display()))?;
        if entry.file_type().is_file() && entry.path().extension().and_then(|ext| ext.to_str()) == Some("ncl") {
            manifest_paths.push(entry.into_path());
        }
    }
    manifest_paths.sort();
    Ok(manifest_paths)
}

fn load_manifest(manifest_path: &Path, schema_source: &str) -> Result<SuiteManifest> {
    let content = fs::read_to_string(manifest_path)
        .with_context(|| format!("failed to read manifest {}", manifest_path.display()))?;
    let wrapped = format!(
        r#"
let schema = {schema_source} in
({content}) | schema.SuiteManifest
"#
    );
    let mut ctx = NickelContext::new().with_source_name(manifest_path.display().to_string());
    let expr = ctx
        .eval_deep(&wrapped)
        .map_err(|err| anyhow::anyhow!(format_nickel_error(err)))
        .with_context(|| format!("failed to evaluate manifest {}", manifest_path.display()))?;
    expr.to_serde::<SuiteManifest>()
        .map_err(|err| anyhow::anyhow!(err))
        .with_context(|| format!("failed to deserialize manifest {}", manifest_path.display()))
}

fn validate_manifest(manifest: &SuiteManifest, manifest_path: &Path, repo_root: &Path) -> Result<()> {
    validate_suite_id(&manifest.id, manifest_path)?;
    let layer = parse_layer(&manifest.layer, manifest_path)?;
    let target_kind = parse_target_kind(&manifest.target.kind, manifest_path)?;

    match layer {
        SuiteLayer::RustIntegration | SuiteLayer::Patchbay => {
            ensure!(
                target_kind == SuiteTargetKind::CargoNextest,
                "{} must use target.kind = cargo-nextest for {} suites",
                manifest_path.display(),
                manifest.layer
            );
        }
        SuiteLayer::Vm => {
            ensure!(
                target_kind == SuiteTargetKind::NixBuild,
                "{} must use target.kind = nix-build for vm suites",
                manifest_path.display()
            );
        }
    }
    match target_kind {
        SuiteTargetKind::CargoNextest => validate_cargo_target(&manifest.target, manifest_path),
        SuiteTargetKind::NixBuild => validate_nix_target(&manifest.target, manifest_path, repo_root),
    }
}

fn validate_suite_id(id: &str, manifest_path: &Path) -> Result<()> {
    ensure!(!id.is_empty(), "{} has an empty suite id", manifest_path.display());
    ensure!(
        id.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '.'),
        "{} has invalid suite id `{}`; use lowercase ASCII, digits, `-`, or `.`",
        manifest_path.display(),
        id
    );
    Ok(())
}

fn validate_cargo_target(target: &RawSuiteTarget, manifest_path: &Path) -> Result<()> {
    ensure!(target.package.is_some(), "{} is missing target.package", manifest_path.display());
    ensure!(target.test.is_some(), "{} is missing target.test", manifest_path.display());
    ensure!(
        !target.register_flake_check,
        "{} cannot enable register_flake_check for cargo-nextest targets",
        manifest_path.display()
    );
    Ok(())
}

fn validate_nix_target(target: &RawSuiteTarget, manifest_path: &Path, repo_root: &Path) -> Result<()> {
    let flake_attr = target
        .flake_attr
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("{} is missing target.flake_attr", manifest_path.display()))?;
    let check_attr = target
        .check_attr
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("{} is missing target.check_attr", manifest_path.display()))?;
    let nix_file = target
        .nix_file
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("{} is missing target.nix_file", manifest_path.display()))?;

    ensure!(
        flake_attr.ends_with(check_attr),
        "{} has mismatched flake/check attrs: `{}` vs `{}`",
        manifest_path.display(),
        flake_attr,
        check_attr
    );

    let nix_path = repo_root.join(nix_file);
    ensure!(nix_path.exists(), "{} references missing nix file {}", manifest_path.display(), nix_path.display());
    validate_package_presets(&target.package_presets, manifest_path)
}

fn validate_package_presets(package_presets: &BTreeMap<String, String>, manifest_path: &Path) -> Result<()> {
    for (name, value) in package_presets {
        match name.as_str() {
            "aspenNodePackage" => ensure!(
                matches!(value.as_str(), "full-aspen-node" | "ci-aspen-node"),
                "{} has unsupported aspenNodePackage preset `{}`",
                manifest_path.display(),
                value
            ),
            "aspenCliPackage" => ensure!(
                matches!(value.as_str(), "full-aspen-cli" | "ci-aspen-cli"),
                "{} has unsupported aspenCliPackage preset `{}`",
                manifest_path.display(),
                value
            ),
            _ => bail!("{} uses unsupported target.package_presets key `{}`", manifest_path.display(), name),
        }
    }
    Ok(())
}

fn validate_generated_registration_keys(
    record: &SuiteInventoryRecord,
    seen_check_attrs: &mut BTreeMap<String, String>,
    seen_flake_attrs: &mut BTreeMap<String, String>,
) -> Result<()> {
    if !record.target.register_flake_check {
        return Ok(());
    }

    if let Some(check_attr) = record.target.check_attr.as_ref()
        && let Some(previous_suite_id) = seen_check_attrs.insert(check_attr.clone(), record.id.clone())
    {
        bail!(
            "duplicate target.check_attr `{}` for suites `{}` and `{}`",
            check_attr,
            previous_suite_id,
            record.id
        );
    }

    if let Some(flake_attr) = record.target.flake_attr.as_ref()
        && let Some(previous_suite_id) = seen_flake_attrs.insert(flake_attr.clone(), record.id.clone())
    {
        bail!(
            "duplicate target.flake_attr `{}` for suites `{}` and `{}`",
            flake_attr,
            previous_suite_id,
            record.id
        );
    }

    Ok(())
}

fn to_inventory_record(
    manifest: SuiteManifest,
    manifest_path: &Path,
    repo_root: &Path,
) -> Result<SuiteInventoryRecord> {
    let SuiteManifest {
        id,
        display_name,
        description,
        layer,
        owner,
        runtime_class,
        prerequisites,
        mut tags,
        target,
    } = manifest;
    let RawSuiteTarget {
        kind,
        package,
        test,
        profile,
        mut features,
        run_ignored,
        flake_attr,
        check_attr,
        nix_file,
        package_presets,
        register_flake_check,
    } = target;

    let layer = parse_layer(&layer, manifest_path)?;
    let runtime_class = parse_runtime_class(&runtime_class, manifest_path)?;
    let mut prerequisites = parse_prerequisites(&prerequisites, manifest_path)?;
    let kind = parse_target_kind(&kind, manifest_path)?;
    let run_ignored = run_ignored.as_deref().map(|value| parse_run_ignored(value, manifest_path)).transpose()?;

    sort_dedup_strings(&mut tags);
    prerequisites.sort();
    prerequisites.dedup();
    sort_dedup_strings(&mut features);

    let manifest_path = repo_relative_path(manifest_path, repo_root)?;

    Ok(SuiteInventoryRecord {
        id,
        display_name,
        description,
        layer,
        owner,
        runtime_class,
        prerequisites,
        tags,
        manifest_path,
        target: SuiteTarget {
            kind,
            package,
            test,
            profile,
            features,
            run_ignored,
            flake_attr,
            check_attr,
            nix_file,
            package_presets,
            register_flake_check,
        },
    })
}

fn parse_layer(value: &str, manifest_path: &Path) -> Result<SuiteLayer> {
    match value {
        "rust-integration" => Ok(SuiteLayer::RustIntegration),
        "patchbay" => Ok(SuiteLayer::Patchbay),
        "vm" => Ok(SuiteLayer::Vm),
        _ => bail!("{} has unsupported layer `{}`", manifest_path.display(), value),
    }
}

fn parse_runtime_class(value: &str, manifest_path: &Path) -> Result<RuntimeClass> {
    match value {
        "real-network" => Ok(RuntimeClass::RealNetwork),
        "linux-namespaces" => Ok(RuntimeClass::LinuxNamespaces),
        "nixos-vm" => Ok(RuntimeClass::NixosVm),
        _ => bail!("{} has unsupported runtime_class `{}`", manifest_path.display(), value),
    }
}

fn parse_prerequisites(values: &[String], manifest_path: &Path) -> Result<Vec<Prerequisite>> {
    values
        .iter()
        .map(|value| match value.as_str() {
            "linux-userns" => Ok(Prerequisite::LinuxUserns),
            "network-access" => Ok(Prerequisite::NetworkAccess),
            "nftables" => Ok(Prerequisite::Nftables),
            "nix-command" => Ok(Prerequisite::NixCommand),
            "traffic-control" => Ok(Prerequisite::TrafficControl),
            _ => bail!("{} has unsupported prerequisite `{}`", manifest_path.display(), value),
        })
        .collect()
}

fn parse_target_kind(value: &str, manifest_path: &Path) -> Result<SuiteTargetKind> {
    match value {
        "cargo-nextest" => Ok(SuiteTargetKind::CargoNextest),
        "nix-build" => Ok(SuiteTargetKind::NixBuild),
        _ => bail!("{} has unsupported target.kind `{}`", manifest_path.display(), value),
    }
}

fn parse_run_ignored(value: &str, manifest_path: &Path) -> Result<RunIgnoredMode> {
    match value {
        "default" => Ok(RunIgnoredMode::Default),
        "all" => Ok(RunIgnoredMode::All),
        "ignored-only" => Ok(RunIgnoredMode::IgnoredOnly),
        _ => bail!("{} has unsupported target.run_ignored `{}`", manifest_path.display(), value),
    }
}

fn repo_relative_path(path: &Path, repo_root: &Path) -> Result<String> {
    let relative = path
        .strip_prefix(repo_root)
        .with_context(|| format!("{} is not under repo root {}", path.display(), repo_root.display()))?;
    Ok(relative.to_string_lossy().replace('\\', "/"))
}

fn sort_dedup_strings(values: &mut Vec<String>) {
    values.sort();
    values.dedup();
}

fn format_nickel_error(err: NickelError) -> String {
    let mut output = Vec::new();
    if err.format(&mut output, ErrorFormat::Text).is_ok() {
        return String::from_utf8_lossy(&output).into_owned();
    }
    format!("{err:?}")
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    const TEST_SCHEMA: &str = include_str!("../../../test-harness/schema.ncl");

    #[test]
    fn load_inventory_normalizes_and_sorts_records() {
        let repo = TestRepo::new();
        repo.write_nix_file("nix/tests/multi-node-kv.nix");
        repo.write_manifest(
            "patchbay/patchbay-fault.ncl",
            r#"
            {
              id = "patchbay-fault",
              layer = "patchbay",
              owner = "networking",
              runtime_class = "linux-namespaces",
              prerequisites = ["traffic-control", "linux-userns", "nftables", "linux-userns"],
              tags = ["recovery", "patchbay", "recovery"],
              target = {
                kind = "cargo-nextest",
                package = "aspen-testing-patchbay",
                test = "patchbay_fault_tests",
                profile = "patchbay",
              },
            }
            "#,
        );
        repo.write_manifest(
            "vm/multi-node-kv.ncl",
            r#"
            {
              id = "multi-node-kv-vm",
              layer = "vm",
              owner = "kv",
              runtime_class = "nixos-vm",
              prerequisites = ["nix-command"],
              tags = ["vm", "kv", "replication", "kv"],
              target = {
                kind = "nix-build",
                flake_attr = "checks.x86_64-linux.multi-node-kv-test",
                check_attr = "multi-node-kv-test",
                nix_file = "nix/tests/multi-node-kv.nix",
                package_presets = {
                  aspenNodePackage = "full-aspen-node",
                  aspenCliPackage = "full-aspen-cli",
                },
                register_flake_check = true,
              },
            }
            "#,
        );

        let inventory = load_inventory(&repo.paths()).expect("inventory should load");
        assert_eq!(inventory.schema_version, SCHEMA_VERSION);
        assert_eq!(inventory.metadata.schema_path, "test-harness/schema.ncl");
        assert_eq!(inventory.metadata.manifest_paths, vec![
            "test-harness/suites/patchbay/patchbay-fault.ncl".to_string(),
            "test-harness/suites/vm/multi-node-kv.ncl".to_string(),
        ]);
        assert_eq!(inventory.metadata.inputs_sha256.len(), 64);
        assert_eq!(inventory.suites.len(), 2);
        assert_eq!(inventory.suites[0].id, "multi-node-kv-vm");
        assert_eq!(inventory.suites[0].tags, vec!["kv".to_string(), "replication".to_string(), "vm".to_string()]);
        assert_eq!(inventory.suites[1].prerequisites, vec![
            Prerequisite::LinuxUserns,
            Prerequisite::Nftables,
            Prerequisite::TrafficControl,
        ]);
    }

    #[test]
    fn duplicate_suite_ids_fail_validation() {
        let repo = TestRepo::new();
        repo.write_manifest(
            "rust/one.ncl",
            r#"
            {
              id = "duplicate-suite",
              layer = "rust-integration",
              owner = "jobs",
              runtime_class = "real-network",
              prerequisites = ["network-access"],
              tags = ["jobs"],
              target = {
                kind = "cargo-nextest",
                package = "aspen",
                test = "job_integration_test",
              },
            }
            "#,
        );
        repo.write_manifest(
            "rust/two.ncl",
            r#"
            {
              id = "duplicate-suite",
              layer = "rust-integration",
              owner = "blob",
              runtime_class = "real-network",
              prerequisites = ["network-access"],
              tags = ["blob"],
              target = {
                kind = "cargo-nextest",
                package = "aspen",
                test = "blob_replication_integration_test",
              },
            }
            "#,
        );

        let err = load_inventory(&repo.paths()).expect_err("duplicate ids must fail");
        assert!(err.to_string().contains("duplicate suite id `duplicate-suite`"));
    }

    #[test]
    fn schema_rejects_unsupported_layer_values() {
        let repo = TestRepo::new();
        repo.write_manifest(
            "rust/invalid-layer.ncl",
            r#"
            {
              id = "invalid-layer",
              layer = "madsim",
              owner = "jobs",
              runtime_class = "real-network",
              prerequisites = ["network-access"],
              tags = ["jobs"],
              target = {
                kind = "cargo-nextest",
                package = "aspen",
                test = "job_integration_test",
              },
            }
            "#,
        );

        let err = load_inventory(&repo.paths()).expect_err("invalid layer must fail");
        assert!(err.to_string().contains("failed to evaluate manifest"));
    }

    #[test]
    fn nix_targets_require_known_package_presets() {
        let repo = TestRepo::new();
        repo.write_nix_file("nix/tests/multi-node-kv.nix");
        repo.write_manifest(
            "vm/multi-node-kv.ncl",
            r#"
            {
              id = "multi-node-kv-vm",
              layer = "vm",
              owner = "kv",
              runtime_class = "nixos-vm",
              prerequisites = ["nix-command"],
              tags = ["kv"],
              target = {
                kind = "nix-build",
                flake_attr = "checks.x86_64-linux.multi-node-kv-test",
                check_attr = "multi-node-kv-test",
                nix_file = "nix/tests/multi-node-kv.nix",
                package_presets = {
                  aspenNodePackage = "full-aspen-node",
                  aspenCliPackage = "unsupported-cli",
                },
                register_flake_check = true,
              },
            }
            "#,
        );

        let err = load_inventory(&repo.paths()).expect_err("invalid package preset must fail");
        assert!(err.to_string().contains("unsupported aspenCliPackage preset `unsupported-cli`"));
    }

    #[test]
    fn duplicate_check_attrs_fail_validation() {
        let repo = TestRepo::new();
        repo.write_nix_file("nix/tests/multi-node-kv.nix");
        repo.write_nix_file("nix/tests/multi-node-blob.nix");
        repo.write_manifest(
            "vm/multi-node-kv.ncl",
            r#"
            {
              id = "multi-node-kv-vm",
              layer = "vm",
              owner = "kv",
              runtime_class = "nixos-vm",
              prerequisites = ["nix-command"],
              tags = ["kv"],
              target = {
                kind = "nix-build",
                flake_attr = "checks.x86_64-linux.alpha-shared-vm-check",
                check_attr = "shared-vm-check",
                nix_file = "nix/tests/multi-node-kv.nix",
                package_presets = {
                  aspenNodePackage = "full-aspen-node",
                  aspenCliPackage = "full-aspen-cli",
                },
                register_flake_check = true,
              },
            }
            "#,
        );
        repo.write_manifest(
            "vm/multi-node-blob.ncl",
            r#"
            {
              id = "multi-node-blob-vm",
              layer = "vm",
              owner = "blob",
              runtime_class = "nixos-vm",
              prerequisites = ["nix-command"],
              tags = ["blob"],
              target = {
                kind = "nix-build",
                flake_attr = "checks.x86_64-linux.beta-shared-vm-check",
                check_attr = "shared-vm-check",
                nix_file = "nix/tests/multi-node-blob.nix",
                package_presets = {
                  aspenNodePackage = "full-aspen-node",
                  aspenCliPackage = "full-aspen-cli",
                },
                register_flake_check = true,
              },
            }
            "#,
        );

        let err = load_inventory(&repo.paths()).expect_err("duplicate check attrs must fail");
        let message = err.to_string();
        assert!(message.contains("duplicate target.check_attr `shared-vm-check`"));
    }

    #[test]
    fn stale_inventory_is_rejected() {
        let repo = TestRepo::new();
        repo.write_manifest(
            "rust/job-integration.ncl",
            r#"
            {
              id = "job-integration",
              layer = "rust-integration",
              owner = "jobs",
              runtime_class = "real-network",
              prerequisites = ["network-access"],
              tags = ["jobs"],
              target = {
                kind = "cargo-nextest",
                package = "aspen",
                test = "job_integration_test",
              },
            }
            "#,
        );

        let original_inventory = load_inventory(&repo.paths()).expect("original inventory should load");
        write_inventory(&original_inventory, &repo.paths().output_path).expect("inventory should write");

        repo.write_manifest(
            "rust/job-integration.ncl",
            r#"
            {
              id = "job-integration",
              layer = "rust-integration",
              owner = "jobs",
              runtime_class = "real-network",
              prerequisites = ["network-access"],
              tags = ["jobs", "updated"],
              target = {
                kind = "cargo-nextest",
                package = "aspen",
                test = "job_integration_test",
              },
            }
            "#,
        );

        let current_inventory = load_inventory(&repo.paths()).expect("updated inventory should load");
        let err = ensure_inventory_is_current(&current_inventory, &repo.paths().output_path)
            .expect_err("stale inventory must fail the freshness check");
        assert!(err.to_string().contains(GENERATED_STALE_MESSAGE));
    }

    struct TestRepo {
        temp_dir: TempDir,
    }

    impl TestRepo {
        fn new() -> Self {
            let temp_dir = TempDir::new().expect("temp dir should exist");
            let repo_root = temp_dir.path();
            fs::create_dir_all(repo_root.join("test-harness/suites")).expect("manifest dir should exist");
            fs::write(repo_root.join("test-harness/schema.ncl"), TEST_SCHEMA).expect("schema should write");
            Self { temp_dir }
        }

        fn paths(&self) -> InventoryPaths {
            let repo_root = self.temp_dir.path().to_path_buf();
            InventoryPaths {
                schema_path: repo_root.join("test-harness/schema.ncl"),
                manifest_root: repo_root.join("test-harness/suites"),
                output_path: repo_root.join("test-harness/generated/inventory.json"),
                repo_root,
            }
        }

        fn write_manifest(&self, relative_path: &str, content: &str) {
            let path = self.temp_dir.path().join("test-harness/suites").join(relative_path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("manifest parent should exist");
            }
            fs::write(path, content).expect("manifest should write");
        }

        fn write_nix_file(&self, relative_path: &str) {
            let path = self.temp_dir.path().join(relative_path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("nix parent should exist");
            }
            fs::write(path, "{ pkgs, ... }: pkgs.runCommand \"stub\" {} \"touch $out\"")
                .expect("nix file should write");
        }
    }
}
