
impl SourceDirectoryRoot {
    pub fn open_existing(source: &std::path::Path) -> crate::error::Result<Self> {
        let metadata = std::fs::symlink_metadata(source).map_err(crate::error::MoltenError::from)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(invalid("materialization source must be a real directory"));
        }
        let dir = cap_std::fs::Dir::open_ambient_dir(source, cap_std::ambient_authority())
            .map_err(crate::error::MoltenError::from)?;
        Ok(Self { dir })
    }

    pub fn from_dir(dir: cap_std::fs::Dir) -> Self {
        Self { dir }
    }

    pub fn read_payloads(
        &self,
        policy: &MaterializationPolicy,
        plan: &MaterializationPlan,
    ) -> crate::error::Result<Vec<MaterializationPayload>> {
        validate_materialization_plan(plan)?;
        let planned_inputs = plan
            .members
            .iter()
            .map(|member| MaterializationMemberInput {
                logical_path: member.logical_path.as_str().to_string(),
                kind: member.kind,
                expected_content_ref: member.expected_content_ref.clone(),
                expected_size: member.expected_size,
            })
            .collect::<Vec<_>>();
        if plan_materialization(policy, &planned_inputs)? != *plan {
            return Err(invalid("source read policy does not match materialization plan"));
        }
        let mut payloads = Vec::with_capacity(plan.members.len());
        for member in &plan.members {
            let bytes = read_regular_file_bounded(&self.dir, member.logical_path.as_path(), policy.max_member_bytes)?;
            verify_payload_bytes(member, &bytes)?;
            payloads.push(MaterializationPayload::new(member.logical_path.as_str(), bytes));
        }
        Ok(payloads)
    }

    pub fn read_path(&self, path: &MaterializationPath, max_bytes: u64) -> crate::error::Result<Vec<u8>> {
        read_regular_file_bounded(&self.dir, path.as_path(), max_bytes)
    }

    pub fn open_subdir(&self, path: &MaterializationPath) -> crate::error::Result<Self> {
        ensure_no_symlink_components(&self.dir, Some(path.as_path()))?;
        let dir = self.dir.open_dir(path.as_path()).map_err(crate::error::MoltenError::from)?;
        Ok(Self { dir })
    }

    pub fn list_regular_files_recursive(
        &self,
        policy: &MaterializationPolicy,
    ) -> crate::error::Result<Vec<MaterializationPath>> {
        validate_policy(policy)?;
        let mut directories = vec![std::path::PathBuf::new()];
        let mut files = Vec::new();
        let mut observed_entries = 0usize;
        while let Some(directory) = directories.pop() {
            let read_path = if directory.as_os_str().is_empty() {
                std::path::Path::new(".")
            } else {
                directory.as_path()
            };
            for entry_result in self.dir.read_dir(read_path).map_err(crate::error::MoltenError::from)? {
                observed_entries = observed_entries
                    .checked_add(1)
                    .ok_or_else(|| invalid("materialization source entry count overflow"))?;
                if observed_entries > policy.max_members {
                    return Err(invalid("materialization source traversal exceeds member bound"));
                }
                let entry = entry_result.map_err(crate::error::MoltenError::from)?;
                let name = entry
                    .file_name()
                    .into_string()
                    .map_err(|_| invalid("materialization source name must be UTF-8"))?;
                let relative = directory.join(name);
                let file_type = entry.file_type().map_err(crate::error::MoltenError::from)?;
                if file_type.is_dir() {
                    directories.push(relative);
                } else if file_type.is_file() {
                    let rendered = logical_path_from_relative_path(&relative)?;
                    crate::bounded::push_bounded(
                        &mut files,
                        MaterializationPath::parse_within(&rendered, policy.max_path_bytes)?,
                        policy.max_members,
                        "materialization source files",
                    )?;
                } else {
                    return Err(invalid("materialization source contains a link or special entry"));
                }
            }
        }
        files.sort();
        Ok(files)
    }
}

pub fn materialize_path(
    destination: &std::path::Path,
    policy: &MaterializationPolicy,
    payloads: &[MaterializationPayload],
) -> crate::error::Result<MaterializationReceipt> {
    let plan = plan_payloads(policy, payloads)?;
    let root = MaterializationRoot::open(destination)?;
    root.materialize(&plan, payloads)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedArchive {
    pub plan: MaterializationPlan,
    pub payloads: Vec<MaterializationPayload>,
}

pub fn write_archive<W: Write>(
    writer: W,
    policy: &MaterializationPolicy,
    payloads: &[MaterializationPayload],
) -> crate::error::Result<W> {
    // r[impl molten.filesystem_materialization.archive_members]
    let plan = plan_payloads(policy, payloads)?;
    let payload_map = validate_payloads(&plan, payloads)?;
    let mut builder = tar::Builder::new(writer);
    for member in &plan.members {
        let bytes = payload_map
            .get(&member.logical_path)
            .ok_or_else(|| invalid("archive payload disappeared after validation"))?;
        let mut header = tar::Header::new_gnu();
        header.set_size(member.expected_size);
        header.set_mode(ARCHIVE_READ_ONLY_MODE);
        header.set_uid(0);
        header.set_gid(0);
        header.set_mtime(0);
        header.set_cksum();
        builder
            .append_data(&mut header, member.logical_path.as_str(), std::io::Cursor::new(*bytes))
            .map_err(crate::error::MoltenError::from)?;
    }
    builder.into_inner().map_err(crate::error::MoltenError::from)
}

pub fn verify_archive<R: Read>(reader: R, policy: &MaterializationPolicy) -> crate::error::Result<VerifiedArchive> {
    // r[impl molten.filesystem_materialization.archive_members]
    validate_policy(policy)?;
    let mut archive = tar::Archive::new(reader);
    let mut payloads = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    let mut total_bytes = 0u64;
    let entries = archive.entries().map_err(crate::error::MoltenError::from)?;
    for entry_result in entries {
        if payloads.len() >= policy.max_members {
            return Err(invalid("archive member count exceeds materialization policy"));
        }
        let mut entry = entry_result.map_err(crate::error::MoltenError::from)?;
        let entry_type = entry.header().entry_type();
        if !entry_type.is_file() {
            return Err(invalid("archive contains a link, directory, or unsupported special entry"));
        }
        let raw_name = entry.path_bytes();
        let name = std::str::from_utf8(raw_name.as_ref()).map_err(|_| invalid("archive member name must be UTF-8"))?;
        let logical_path = MaterializationPath::parse_within(name, policy.max_path_bytes)?;
        if policy.reserved_top_level_names.iter().any(|reserved| reserved == logical_path.top_level()) {
            return Err(invalid("archive member uses a reserved materialization name"));
        }
        if !seen.insert(logical_path.clone()) {
            return Err(invalid(format!("duplicate normalized archive member: {}", logical_path.as_str())));
        }
        let declared_size = entry.header().size().map_err(crate::error::MoltenError::from)?;
        if declared_size > policy.max_member_bytes {
            return Err(invalid("archive member exceeds materialization byte bound"));
        }
        total_bytes =
            total_bytes.checked_add(declared_size).ok_or_else(|| invalid("archive total byte count overflow"))?;
        if total_bytes > policy.max_total_bytes {
            return Err(invalid("archive total bytes exceed materialization policy"));
        }
        let bytes = read_bounded(&mut entry, policy.max_member_bytes)?;
        if u64::try_from(bytes.len()).map_err(|_| invalid("archive member size does not fit u64"))? != declared_size {
            return Err(invalid("archive member byte count does not match header"));
        }
        payloads.push(MaterializationPayload::new(logical_path.as_str(), bytes));
    }
    let plan = plan_payloads(policy, &payloads)?;
    Ok(VerifiedArchive { plan, payloads })
}

pub fn create_explicit_output_file(path: &std::path::Path) -> crate::error::Result<std::fs::File> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    let leaf = path.file_name().ok_or_else(|| invalid("explicit output file path has no file name"))?;
    std::fs::create_dir_all(parent).map_err(crate::error::MoltenError::from)?;
    let parent_dir = cap_std::fs::Dir::open_ambient_dir(parent, cap_std::ambient_authority())
        .map_err(crate::error::MoltenError::from)?;
    let mut options = cap_std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true).follow(cap_fs_ext::FollowSymlinks::No);
    let file = parent_dir
        .open_with(std::path::Path::new(leaf), &options)
        .map_err(crate::error::MoltenError::from)?;
    if !file.metadata().map_err(crate::error::MoltenError::from)?.is_file() {
        return Err(invalid("explicit output leaf must be a regular file"));
    }
    Ok(file.into_std())
}

pub fn open_explicit_input_file(path: &std::path::Path) -> crate::error::Result<std::fs::File> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    let leaf = path.file_name().ok_or_else(|| invalid("explicit input file path has no file name"))?;
    let parent_dir = cap_std::fs::Dir::open_ambient_dir(parent, cap_std::ambient_authority())
        .map_err(crate::error::MoltenError::from)?;
    let mut options = cap_std::fs::OpenOptions::new();
    options.read(true).follow(cap_fs_ext::FollowSymlinks::No);
    let file = parent_dir
        .open_with(std::path::Path::new(leaf), &options)
        .map_err(crate::error::MoltenError::from)?;
    if !file.metadata().map_err(crate::error::MoltenError::from)?.is_file() {
        return Err(invalid("explicit input leaf must be a regular file"));
    }
    Ok(file.into_std())
}

fn materialization_plan_value(
    policy: &MaterializationPolicy,
    members: &[MaterializationMember],
    total_bytes: u64,
) -> crate::error::Result<preserves::IOValue> {
    let member_count =
        u64::try_from(members.len()).map_err(|_| invalid("materialization member count does not fit u64"))?;
    let maximum_members =
        u64::try_from(policy.max_members).map_err(|_| invalid("materialization member bound does not fit u64"))?;
    let maximum_path_bytes =
        u64::try_from(policy.max_path_bytes).map_err(|_| invalid("materialization path bound does not fit u64"))?;
    Ok(crate::preserves_rail::record("filesystem-materialization-plan-v1", vec![
        crate::preserves_rail::string(MATERIALIZATION_PLAN_SCHEMA),
        crate::preserves_rail::record("profile", vec![crate::preserves_rail::string(&policy.profile)]),
        crate::preserves_rail::record("replacement", vec![crate::preserves_rail::string(policy.replacement.as_str())]),
        crate::preserves_rail::record("members", vec![crate::preserves_rail::sequence(
            members
                .iter()
                .map(|member| {
                    crate::preserves_rail::record("member", vec![
                        crate::preserves_rail::string(member.logical_path.as_str()),
                        crate::preserves_rail::string(member.kind.as_str()),
                        crate::preserves_rail::string(&member.expected_content_ref),
                        crate::preserves_rail::u64_value(member.expected_size),
                    ])
                })
                .collect(),
        )]),
        crate::preserves_rail::record("summary", vec![
            crate::preserves_rail::u64_value(member_count),
            crate::preserves_rail::u64_value(total_bytes),
        ]),
        crate::preserves_rail::record("reserved-top-level", vec![crate::preserves_rail::sequence(
            policy.reserved_top_level_names.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("bounds", vec![
            crate::preserves_rail::u64_value(maximum_members),
            crate::preserves_rail::u64_value(policy.max_member_bytes),
            crate::preserves_rail::u64_value(policy.max_total_bytes),
            crate::preserves_rail::u64_value(maximum_path_bytes),
        ]),
    ]))
}
