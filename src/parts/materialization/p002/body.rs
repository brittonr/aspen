
impl MaterializationRoot {
    pub fn open(destination: &std::path::Path) -> crate::error::Result<Self> {
        // r[impl molten.filesystem_materialization.root]
        match std::fs::symlink_metadata(destination) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err(invalid("materialization destination must be a real directory"));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                std::fs::create_dir_all(destination).map_err(crate::error::MoltenError::from)?;
            }
            Err(error) => return Err(crate::error::MoltenError::from(error)),
        }
        let dir = cap_std::fs::Dir::open_ambient_dir(destination, cap_std::ambient_authority())
            .map_err(crate::error::MoltenError::from)?;
        Ok(Self::from_dir(dir))
    }

    pub fn from_dir(dir: cap_std::fs::Dir) -> Self {
        Self {
            inner: std::sync::Arc::new(MaterializationRootInner { dir }),
        }
    }

    pub fn stage(
        &self,
        plan: &MaterializationPlan,
        payloads: &[MaterializationPayload],
    ) -> crate::error::Result<StagedMaterialization> {
        validate_materialization_plan(plan)?;
        let payload_map = validate_payloads(plan, payloads)?;
        let stage_path = stage_path(plan)?;
        let stage_result = self.stage_inner(plan, &payload_map, &stage_path);
        if let Err(error) = stage_result {
            let cleanup_result = remove_tree_if_present(&self.inner.dir, &stage_path);
            return match cleanup_result {
                Ok(()) => Err(error),
                Err(cleanup_error) => Err(invalid(format!(
                    "materialization stage failed: {error}; stage cleanup failed: {cleanup_error}"
                ))),
            };
        }
        Ok(StagedMaterialization {
            root: std::sync::Arc::clone(&self.inner),
            plan_ref: plan.plan_ref.clone(),
            stage_path,
        })
    }

    pub fn commit(
        &self,
        plan: &MaterializationPlan,
        staged: &StagedMaterialization,
    ) -> crate::error::Result<MaterializationReceipt> {
        self.commit_inner(plan, staged, None)
    }

    fn commit_inner(
        &self,
        plan: &MaterializationPlan,
        staged: &StagedMaterialization,
        fail_after_publications: Option<usize>,
    ) -> crate::error::Result<MaterializationReceipt> {
        // r[impl molten.filesystem_materialization.commit]
        validate_materialization_plan(plan)?;
        if !std::sync::Arc::ptr_eq(&self.inner, &staged.root) {
            return Err(invalid("staged materialization belongs to a different destination root"));
        }
        if staged.plan_ref != plan.plan_ref {
            return Err(invalid("staged materialization plan identity is stale or mismatched"));
        }
        self.preflight_publication(plan)?;
        let mut created_final_directories = Vec::new();
        for member in &plan.members {
            if let Err(error) = create_directory_tree_recording(
                &self.inner.dir,
                member.logical_path.as_path().parent(),
                &mut created_final_directories,
            ) {
                return Err(setup_failure(&self.inner.dir, &created_final_directories, error));
            }
            let backup_path = staged.stage_path.join(STAGING_BACKUP_DIRECTORY).join(member.logical_path.as_path());
            if let Err(error) = create_directory_tree(&self.inner.dir, backup_path.parent()) {
                return Err(setup_failure(&self.inner.dir, &created_final_directories, error));
            }
        }
        let mut states = Vec::with_capacity(plan.members.len());
        for member in &plan.members {
            let final_path = member.logical_path.as_path().to_path_buf();
            let backup_path = staged.stage_path.join(STAGING_BACKUP_DIRECTORY).join(member.logical_path.as_path());
            let mut state = PublicationState {
                final_path: final_path.clone(),
                backup_path: None,
                published: false,
            };
            let existing = match entry_kind(&self.inner.dir, &final_path) {
                Ok(existing) => existing,
                Err(error) => {
                    return Err(publication_failure(
                        &self.inner.dir,
                        &state,
                        &states,
                        &created_final_directories,
                        error,
                    ));
                }
            };
            match (plan.replacement, existing) {
                (ReplacementPolicy::NoReplace, None) | (ReplacementPolicy::ReplaceRegularFiles, None) => {}
                (ReplacementPolicy::NoReplace, Some(_)) => {
                    return Err(publication_failure(
                        &self.inner.dir,
                        &state,
                        &states,
                        &created_final_directories,
                        invalid("no-replace materialization target appeared during publication"),
                    ));
                }
                (ReplacementPolicy::ReplaceRegularFiles, Some(MaterializationMemberKind::RegularFile)) => {
                    if let Err(error) = self.inner.dir.rename(&final_path, &self.inner.dir, &backup_path) {
                        return Err(publication_failure(
                            &self.inner.dir,
                            &state,
                            &states,
                            &created_final_directories,
                            crate::error::MoltenError::from(error),
                        ));
                    }
                    state.backup_path = Some(backup_path);
                }
                (ReplacementPolicy::ReplaceRegularFiles, Some(_)) => {
                    return Err(publication_failure(
                        &self.inner.dir,
                        &state,
                        &states,
                        &created_final_directories,
                        invalid("replacement target changed to a link or special entry during publication"),
                    ));
                }
            }
            let staged_file = staged.stage_path.join(STAGING_TREE_DIRECTORY).join(member.logical_path.as_path());
            if let Err(error) = self.inner.dir.hard_link(&staged_file, &self.inner.dir, &final_path) {
                return Err(publication_failure(
                    &self.inner.dir,
                    &state,
                    &states,
                    &created_final_directories,
                    crate::error::MoltenError::from(error),
                ));
            }
            state.published = true;
            states.push(state);
            if let Err(error) = self.inner.dir.remove_file(&staged_file) {
                return Err(rollback_failure(
                    &self.inner.dir,
                    &states,
                    &created_final_directories,
                    crate::error::MoltenError::from(error),
                ));
            }
            if fail_after_publications.is_some_and(|limit| states.len() == limit) {
                return Err(rollback_failure(
                    &self.inner.dir,
                    &states,
                    &created_final_directories,
                    invalid("injected materialization publication failure"),
                ));
            }
        }
        if let Err(error) = verify_published_members(&self.inner.dir, plan) {
            return Err(rollback_failure(&self.inner.dir, &states, &created_final_directories, error));
        }
        remove_tree_if_present(&self.inner.dir, &staged.stage_path)?;
        build_materialization_receipt(plan)
    }

    pub fn abort(&self, staged: &StagedMaterialization) -> crate::error::Result<()> {
        if !std::sync::Arc::ptr_eq(&self.inner, &staged.root) {
            return Err(invalid("cannot abort a stage owned by another materialization root"));
        }
        remove_tree_if_present(&self.inner.dir, &staged.stage_path)
    }

    pub fn materialize(
        &self,
        plan: &MaterializationPlan,
        payloads: &[MaterializationPayload],
    ) -> crate::error::Result<MaterializationReceipt> {
        let staged = self.stage(plan, payloads)?;
        match self.commit(plan, &staged) {
            Ok(receipt) => Ok(receipt),
            Err(error) => {
                let abort_result = self.abort(&staged);
                match abort_result {
                    Ok(()) => Err(error),
                    Err(abort_error) => Err(invalid(format!(
                        "materialization commit failed: {error}; stage quarantine cleanup failed: {abort_error}"
                    ))),
                }
            }
        }
    }

    pub fn read(&self, path: &MaterializationPath) -> crate::error::Result<Vec<u8>> {
        read_regular_file_bounded(&self.inner.dir, path.as_path(), DEFAULT_MAX_MATERIALIZATION_MEMBER_BYTES)
    }

    fn stage_inner(
        &self,
        plan: &MaterializationPlan,
        payloads: &std::collections::BTreeMap<MaterializationPath, &[u8]>,
        stage_path: &std::path::Path,
    ) -> crate::error::Result<()> {
        create_staging_root(&self.inner.dir, stage_path)?;
        for member in &plan.members {
            let bytes = payloads
                .get(&member.logical_path)
                .ok_or_else(|| invalid("materialization payload disappeared after validation"))?;
            let staged_path = stage_path.join(STAGING_TREE_DIRECTORY).join(member.logical_path.as_path());
            create_directory_tree(&self.inner.dir, staged_path.parent())?;
            write_create_new(&self.inner.dir, &staged_path, bytes)?;
            verify_member_bytes(member, &self.inner.dir, &staged_path)?;
        }
        Ok(())
    }

    fn preflight_publication(&self, plan: &MaterializationPlan) -> crate::error::Result<()> {
        for member in &plan.members {
            let final_path = member.logical_path.as_path();
            ensure_no_symlink_components(&self.inner.dir, final_path.parent())?;
            match entry_kind(&self.inner.dir, final_path)? {
                None => {}
                Some(MaterializationMemberKind::RegularFile)
                    if plan.replacement == ReplacementPolicy::ReplaceRegularFiles => {}
                Some(kind) if plan.replacement == ReplacementPolicy::ReplaceRegularFiles => {
                    return Err(invalid(format!(
                        "materialization replacement target {} has unsupported kind {}",
                        member.logical_path.as_str(),
                        kind.as_str()
                    )));
                }
                Some(_) => {
                    return Err(invalid(format!(
                        "materialization no-replace target already exists: {}",
                        member.logical_path.as_str()
                    )));
                }
            }
        }
        Ok(())
    }
}

pub struct StagedMaterialization {
    root: std::sync::Arc<MaterializationRootInner>,
    plan_ref: String,
    stage_path: std::path::PathBuf,
}

impl std::fmt::Debug for StagedMaterialization {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StagedMaterialization")
            .field("plan_ref", &self.plan_ref)
            .finish_non_exhaustive()
    }
}

pub struct SourceDirectoryRoot {
    dir: cap_std::fs::Dir,
}

impl std::fmt::Debug for SourceDirectoryRoot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("SourceDirectoryRoot").finish_non_exhaustive()
    }
}
