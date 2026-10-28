use std::io::Write;
use std::os::fd::AsRawFd;

use super::*;

const PUBLICATION: &str = "tmp/aspen-mantle-boundary-witness-20261027/producer-output";
const BUNDLE_BYTES: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/wasm-component-runtime/producer-tree-handoff/materialization-bundle.json"
));

fn bundle() -> mantle_wasm_consumer_verifier::MaterializationBundle {
    serde_json::from_slice(BUNDLE_BYTES).expect("unchanged real producer bundle")
}

fn install(root: &cap_std::fs::Dir) {
    for (path, bytes) in super::PRODUCER_FILES {
        let relative = format!("{PUBLICATION}/{path}");
        staging::create_member(root, &relative)
            .expect("fresh fixture member")
            .write_all(bytes)
            .expect("unchanged publication bytes");
    }
}

fn required(bundle: &mantle_wasm_consumer_verifier::MaterializationBundle) -> Vec<RequiredMember> {
    mantle_wasm_consumer_verifier::facade::declared_member_objects(bundle)
        .into_iter()
        .map(|(role, object)| RequiredMember {
            role,
            logical_path: object.logical_path.clone(),
            digest: object.digest_blake3.to_string(),
            size: object.size_bytes,
        })
        .collect()
}

fn measure(
    root: &cap_std::fs::Dir,
    stage: &cap_std::fs::Dir,
    bundle: &mantle_wasm_consumer_verifier::MaterializationBundle,
    members: &[RequiredMember],
) -> Result<measurement::MeasuredMembers> {
    measurement::measure_members(
        root,
        stage,
        members,
        measurement::Selection {
            component_path: &bundle.final_portable.logical_path,
            wit_path: &bundle.wit_inputs[0].logical_path,
            profile: &crate::wasm_component::supported().expect("profile byte limits"),
        },
        bundle,
    )
}

// r[verify molten.wasm_component.materialization]
// r[verify molten.wasm_component_adoption.validation]
#[test]
fn genuine_producer_tree_stages_without_aliasing_and_requires_complete_report_agreement() {
    let workspace = crate::test_support::Workspace::new("published-genuine-tree").expect("workspace");
    let input = workspace.input().expect("source capability");
    let output = workspace.output().expect("private staging parent");
    install(input.dir());
    let bundle = bundle();
    let structural = mantle_wasm_consumer_verifier::verify_consumer_bundle(bundle.clone());
    require_structural(&structural).expect("unchanged producer structural report");
    let members = required(&bundle);
    let source_file = format!("{}/src/app.rs", bundle.source_closure.logical_path.trim_start_matches('/'));
    let original_source = input.dir().read(&source_file).expect("original source bytes");
    let component = input.dir().read(bundle.final_portable.logical_path.trim_start_matches('/')).expect("component");
    let wit = input.dir().read(bundle.wit_inputs[0].logical_path.trim_start_matches('/')).expect("WIT");
    staging::with_private_stage(output.dir(), "tree-stage", |stage| {
        let measured = measure(input.dir(), stage, &bundle, &members)?;
        assert_eq!(measured.component_bytes, component);
        assert_eq!(measured.wit_bytes, wit);
        // Mutating the original tree after capture cannot alter staged bytes,
        // selected executable bytes, or the unchanged published bundle identity.
        input.dir().write(&source_file, b"changed after private copy").expect("source mutation");
        assert_eq!(stage.read(&source_file).expect("private source bytes"), original_source);
        let verifier = staging::consumer_root(stage)?;
        let complete = verifier.verify_bundle(bundle.clone()).expect("published Bytes report");
        require_complete(&structural, &complete, &measured.observations)?;
        assert_eq!(complete.bundle_identity_blake3, structural.bundle_identity_blake3);
        assert_eq!(measured.component_bytes, component);
        assert_eq!(measured.wit_bytes, wit);

        let mut incomplete = complete.clone();
        incomplete.layers_completed.pop();
        assert!(require_complete(&structural, &incomplete, &measured.observations).is_err());
        let mut disagreement = measured.observations.clone();
        disagreement[0].size_bytes += 1;
        assert!(require_complete(&structural, &complete, &disagreement).is_err());

        stage.write(&source_file, b"tampered private source").expect("stage tamper");
        let tampered = verifier.verify_bundle(bundle.clone()).expect("tamper report");
        assert_ne!(tampered.status, mantle_wasm_consumer_verifier::ConsumerVerificationStatus::Verified);
        assert!(require_complete(&structural, &tampered, &measured.observations).is_err());
        Ok(())
    })
    .expect("private stage removed after observation");
    assert!(!output.dir().try_exists("tree-stage").expect("lease removed"));
}

fn mutate_source(
    input: &cap_std::fs::Dir,
    mutation: &str,
    source: &str,
    file: &str,
) -> Option<std::os::unix::net::UnixListener> {
    let mut socket = None;
    match mutation {
        "file-drift" => input.write(file, b"changed").expect("source drift"),
        // Empty directories participate in producer identity even though
        // they contribute no content bytes to its declared size.
        "empty-directory" => input.create_dir(format!("{source}/empty")).expect("empty dir"),
        "missing-tree" => input.remove_dir_all(source).expect("remove tree"),
        "missing-child" => input.remove_file(file).expect("remove child"),
        "source-leaf-symlink" => {
            let parent = source.rsplit_once('/').expect("source parent").0;
            input.rename(source, input, format!("{parent}/real-source")).expect("move source");
            input.symlink("real-source", source).expect("source leaf symlink");
        }
        "source-parent-symlink" => {
            let parent = source.rsplit_once('/').expect("source parent").0;
            let grandparent = parent.rsplit_once('/').expect("source grandparent").0;
            input.rename(parent, input, format!("{grandparent}/real-inputs")).expect("move parent");
            input.symlink("real-inputs", parent).expect("source parent symlink");
        }
        "child-symlink" => {
            input.rename(file, input, format!("{source}/src/real-app.rs")).expect("move child");
            input.symlink("real-app.rs", file).expect("source child symlink");
        }
        "nested-parent-symlink" => {
            input
                .rename(format!("{source}/src"), input, format!("{source}/real-src"))
                .expect("move nested parent");
            input.symlink("real-src", format!("{source}/src")).expect("nested parent symlink");
        }
        "special-node" => {
            // Bind through an explicitly held root descriptor, not an
            // ambient discovered workspace path. Keep the socket live.
            let source_dir = input.open_dir(source).expect("source directory capability");
            let path = format!("/proc/self/fd/{}/socket", source_dir.as_raw_fd());
            socket = Some(std::os::unix::net::UnixListener::bind(path).expect("special source node"));
        }
        "over-byte-bound" => {
            input
                .open_with(file, cap_std::fs::OpenOptions::new().write(true))
                .expect("source file")
                .set_len(536_870_913)
                .expect("sparse over-bound source");
        }
        "over-depth-bound" => {
            let mut path = format!("{source}/deep");
            for _ in 0..65 {
                input.create_dir(&path).expect("deep directory");
                path.push_str("/deep");
            }
        }
        _ => unreachable!("closed mutation corpus"),
    }
    socket
}

// r[verify molten.wasm_component.materialization]
// r[verify molten.wasm_component.fixtures]
#[test]
fn source_tree_drift_missing_symlinks_and_bounds_deny_and_clean_private_staging() {
    for mutation in [
        "file-drift",
        "empty-directory",
        "missing-tree",
        "missing-child",
        "source-leaf-symlink",
        "source-parent-symlink",
        "child-symlink",
        "nested-parent-symlink",
        "special-node",
        "over-byte-bound",
        "over-depth-bound",
    ] {
        let workspace = crate::test_support::Workspace::new("published-tree-denials").expect("workspace");
        let input = workspace.input().expect("source capability");
        let output = workspace.output().expect("private staging parent");
        output.dir().write("unrelated", b"keep").expect("unrelated sibling");
        install(input.dir());
        let bundle = bundle();
        let source = bundle.source_closure.logical_path.trim_start_matches('/');
        let file = format!("{source}/src/app.rs");
        let socket = mutate_source(input.dir(), mutation, source, &file);
        let members = required(&bundle);
        let result = staging::with_private_stage(output.dir(), "tree-stage", |stage| {
            measure(input.dir(), stage, &bundle, &members)
        });
        assert!(result.is_err(), "{mutation}");
        assert!(!output.dir().try_exists("tree-stage").expect("lease removed"), "{mutation}");
        assert_eq!(output.dir().read("unrelated").expect("unrelated survives"), b"keep", "{mutation}");
        drop(socket);
    }
}

// r[verify molten.wasm_component.materialization]
#[test]
fn directory_source_does_not_grant_directory_acceptance_to_another_role() {
    let workspace = crate::test_support::Workspace::new("published-tree-role-alias").expect("workspace");
    let input = workspace.input().expect("source capability");
    let output = workspace.output().expect("private staging parent");
    install(input.dir());
    let bundle = bundle();
    let mut members = required(&bundle);
    members.push(RequiredMember {
        role: "stage-artifact",
        logical_path: bundle.source_closure.logical_path.clone(),
        digest: bundle.source_closure.digest_blake3.to_string(),
        size: bundle.source_closure.size_bytes,
    });
    assert!(
        staging::with_private_stage(output.dir(), "tree-stage", |stage| {
            measure(input.dir(), stage, &bundle, &members)
        })
        .is_err()
    );
    assert!(!output.dir().try_exists("tree-stage").expect("lease removed"));
}

// r[verify molten.wasm_component.materialization]
#[test]
fn later_member_failure_cleans_the_successfully_copied_source_tree() {
    let workspace = crate::test_support::Workspace::new("published-tree-partial-cleanup").expect("workspace");
    let input = workspace.input().expect("source capability");
    let output = workspace.output().expect("private staging parent");
    install(input.dir());
    let bundle = bundle();
    let members = required(&bundle);
    input.dir().remove_file(bundle.lock.logical_path.trim_start_matches('/')).expect("missing lock");
    assert!(
        staging::with_private_stage(output.dir(), "tree-stage", |stage| {
            measure(input.dir(), stage, &bundle, &members)
        })
        .is_err()
    );
    assert!(!output.dir().try_exists("tree-stage").expect("copied source tree and lease removed"));
    assert!(input.dir().is_dir(bundle.source_closure.logical_path.trim_start_matches('/')));
}
