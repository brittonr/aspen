use super::*;

fn rebind_provenance(entry: &mut Entry, max_bytes: usize) {
    let report = report_manifest(&entry.report, max_bytes).expect("retained report");
    let provenance = encode_parts(
        PROVENANCE_DOMAIN,
        &[
            &entry.specialization_manifest,
            &entry.subject_manifest,
            &entry.program_manifest,
            &entry.artifact_digest,
            &report,
        ],
        max_bytes,
    )
    .expect("retained provenance");
    entry.provenance = digest(&TestIds, &provenance);
}

#[test]
fn source_dependencies_bindings_interfaces_and_assumptions_do_not_reuse() {
    let base = graph(&TestIds, false, 7, &[11, 12], false, false, false);
    let changed_graphs = [
        graph(&TestIds, true, 7, &[11, 12], false, false, false),
        graph(&TestIds, false, 7, &[11, 13], false, false, false),
        graph(&TestIds, false, 8, &[11, 12], false, false, false),
    ];
    for changed in changed_graphs {
        let mut session = Session::default();
        assert_eq!(
            run(&mut session, &TestIds, &base, &[Some(5), None])
                .unwrap()
                .status,
            Status::MissStored
        );
        assert_ne!(
            run(&mut session, &TestIds, &changed, &[Some(5), None])
                .unwrap()
                .status,
            Status::Hit
        );
    }

    let mut static_value = Session::default();
    run(&mut static_value, &TestIds, &base, &[Some(5), None]).unwrap();
    assert_ne!(
        run(&mut static_value, &TestIds, &base, &[Some(6), None])
            .unwrap()
            .status,
        Status::Hit
    );

    let three = graph(&TestIds, false, 7, &[11, 12], true, false, false);
    let mut positions = Session::default();
    run(&mut positions, &TestIds, &three, &[Some(5), None, None]).unwrap();
    assert_ne!(
        run(&mut positions, &TestIds, &three, &[None, Some(5), None])
            .unwrap()
            .status,
        Status::Hit
    );

    let typed = graph(&TestIds, false, 7, &[11, 12], false, true, false);
    let mut interface = Session::default();
    run(&mut interface, &TestIds, &base, &[Some(5), None]).unwrap();
    assert_ne!(
        run(&mut interface, &TestIds, &typed, &[Some(5), None, None])
            .unwrap()
            .status,
        Status::Hit
    );

    let assumed = graph(&TestIds, false, 7, &[11, 12], false, false, true);
    let mut assumptions = Session::default();
    run(&mut assumptions, &TestIds, &base, &[Some(5), None]).unwrap();
    let refusal = run(&mut assumptions, &TestIds, &assumed, &[Some(5), None])
        .expect_err("an assumption-bearing graph is outside the profile");
    assert_eq!(refusal.code, crate::diag::Code::Profile);
    assert_eq!(assumptions.len(), 1, "the refusal was not cached");
}

#[test]
fn exact_recipe_bytes_change_specialization_identity() {
    let original = graph(&TestIds, false, 7, &[11, 12], false, false, false);
    let canonical = crate::format::graph::print(original.graph());
    let reformatted = alloc::format!("# same accepted graph\n{canonical}");
    let mut session = Session::default();
    let static_inputs = [Some(5), None];
    let budgets = crate::check::Budgets::default();
    let request = crate::specialize::Request {
        static_inputs: &static_inputs,
        budgets: &budgets,
    };
    let first = session
        .specialize(super::Invocation::new(
            &TestIds,
            &original,
            canonical.as_bytes(),
            request,
            noble_contracts::Limits::default(),
        ))
        .expect("canonical recipe specializes");
    let second = session
        .specialize(super::Invocation::new(
            &TestIds,
            &original,
            reformatted.as_bytes(),
            request,
            noble_contracts::Limits::default(),
        ))
        .expect("reformatted recipe specializes");
    assert_eq!(first.status, Status::MissStored);
    assert_eq!(second.status, Status::MissStored);
    assert_ne!(first.specialization_key, second.specialization_key);
}

#[test]
fn semantic_and_specializer_option_changes_miss_in_full_key() {
    let original = graph(&TestIds, false, 7, &[11, 12], false, false, false);
    let recipe = crate::format::graph::print(original.graph());
    let map = [Some(5), None];
    let budgets = crate::check::Budgets::default();
    let limits = noble_contracts::Limits::default();
    let request = crate::specialize::Request {
        static_inputs: &map,
        budgets: &budgets,
    };
    let expected = specialization_manifest_with(SpecializationManifestInput {
        original: &original,
        recipe: recipe.as_bytes(),
        request,
        frontend_limits: limits,
        max_bytes: usize::MAX,
        epoch: SpecializationEpoch {
            semantic_revision: b"ngraph-sem/0",
            specializer_revision: b"ngraph-pe-specializer/2",
            options: b"",
        },
    })
    .expect("fixture manifest");
    for (semantic, revision, options) in [
        (
            b"ngraph-sem/1".as_slice(),
            b"ngraph-pe-specializer/2".as_slice(),
            b"".as_slice(),
        ),
        (
            b"ngraph-sem/0".as_slice(),
            b"ngraph-pe-specializer/3".as_slice(),
            b"".as_slice(),
        ),
        (
            b"ngraph-sem/0".as_slice(),
            b"ngraph-pe-specializer/2".as_slice(),
            b"alternate-fold-option".as_slice(),
        ),
    ] {
        let changed = specialization_manifest_with(SpecializationManifestInput {
            original: &original,
            recipe: recipe.as_bytes(),
            request,
            frontend_limits: limits,
            max_bytes: usize::MAX,
            epoch: SpecializationEpoch {
                semantic_revision: semantic,
                specializer_revision: revision,
                options,
            },
        })
        .expect("changed manifest");
        assert_ne!(expected, changed);
        let mut session = Session::default();
        assert_eq!(
            run(&mut session, &TestIds, &original, &map).unwrap().status,
            Status::MissStored
        );
        let max_bytes = session.limits.manifest_bytes;
        let entry = &mut session.entries[0];
        entry.specialization_manifest = changed;
        entry.specialization_key = digest(&TestIds, &entry.specialization_manifest);
        rebind_provenance(entry, max_bytes);
        assert_eq!(
            run(&mut session, &TestIds, &original, &map).unwrap().status,
            Status::MissStored,
            "an entry from another semantic or specializer epoch is not a hit"
        );
    }
}

#[test]
fn backend_flags_runtime_abi_and_compiler_epochs_change_build_key() {
    let checked = graph(&TestIds, false, 7, &[11, 12], false, false, false);
    let static_inputs = [Some(5), None];
    let budgets = crate::check::Budgets::default();
    let request = crate::specialize::Request {
        static_inputs: &static_inputs,
        budgets: &budgets,
    };
    let residual = crate::specialize::specialize(&TestIds, &checked, &request).expect("residual");
    let graph_text = crate::format::graph::print(residual.checked.graph());
    let source = crate::specialize::emit_source(&residual.checked).expect("source");
    let budgets = crate::check::Budgets::default();
    let frontend = noble_contracts::Limits::default();
    let standard = BuildEpoch {
        target: BUILD_TARGET,
        representation: BUILD_REPRESENTATION,
        backend: BUILD_BACKEND,
        backend_flags: BUILD_BACKEND_FLAGS,
        runtime: BUILD_RUNTIME,
        abi_revision: BUILD_ABI_REVISION,
        compiler_epoch: BUILD_COMPILER_EPOCH,
    };
    let base = BuildManifestBase {
        residual: residual.checked.graph(),
        graph_text: graph_text.as_bytes(),
        source: source.as_bytes(),
        budgets: &budgets,
        frontend_limits: frontend,
        max_bytes: usize::MAX,
    };
    let expected = build_manifest_with_epoch(BuildManifestInput {
        base,
        epoch: standard,
    })
    .expect("build manifest");
    let mut epochs = [standard; 7];
    epochs[0].target = b"other-target";
    epochs[1].representation = b"other-representation";
    epochs[2].backend = b"other-backend";
    epochs[3].backend_flags = b"different-flags";
    epochs[4].runtime = b"other-runtime";
    epochs[5].abi_revision = b"other-abi";
    epochs[6].compiler_epoch = b"other-compiler-epoch";
    for epoch in epochs {
        let changed = build_manifest_with_epoch(BuildManifestInput { base, epoch })
            .expect("changed build manifest");
        assert_ne!(expected, changed);
        let mut session = Session::default();
        assert_eq!(
            run(&mut session, &TestIds, &checked, &static_inputs)
                .unwrap()
                .status,
            Status::MissStored
        );
        let max_bytes = session.limits.manifest_bytes;
        let entry = &mut session.entries[0];
        entry.program_manifest = changed;
        entry.program_key = digest(&TestIds, &entry.program_manifest);
        rebind_provenance(entry, max_bytes);
        assert_eq!(
            run(&mut session, &TestIds, &checked, &static_inputs)
                .unwrap()
                .status,
            Status::CorruptRebuilt,
            "an entry built under another target, backend, runtime or compiler epoch is rejected"
        );
    }
}
