
impl StorageRoot {
    pub fn open(kind: Kind, root: &Path) -> Result<Self> {
        std::fs::create_dir_all(root).map_err(MoltenError::from)?;
        Self::open_existing(kind, root)
    }

    pub fn open_existing(kind: Kind, root: &Path) -> Result<Self> {
        let dir = cap_std::fs::Dir::open_ambient_dir(root, cap_std::ambient_authority()).map_err(MoltenError::from)?;
        Ok(Self { kind, dir })
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    #[doc(hidden)]
    pub fn try_clone_dir(&self) -> Result<cap_std::fs::Dir> {
        self.dir.try_clone().map_err(MoltenError::from)
    }

    pub(crate) fn from_dir(kind: Kind, dir: cap_std::fs::Dir) -> Self {
        Self { kind, dir }
    }

    fn open_subdir(&self, kind: Kind, path: &Locator) -> Result<Self> {
        self.create_dir_all(path)?;
        let dir = self.dir.open_dir(path.as_path()).map_err(MoltenError::from)?;
        Ok(Self { kind, dir })
    }

    fn share_authority_as(&self, kind: Kind) -> Result<Self> {
        let dir = self.dir.try_clone().map_err(MoltenError::from)?;
        Ok(Self { kind, dir })
    }

    pub fn create_dir_all(&self, path: &Locator) -> Result<()> {
        self.dir.create_dir_all(path.as_path()).map_err(MoltenError::from)
    }

    pub fn read(&self, path: &Locator) -> Result<Vec<u8>> {
        if self.entry_kind(path)? != MemberKind::File {
            return Err(MoltenError::invalid_harness(format!(
                "local store read leaf {} must be a regular file",
                path.display()
            )));
        }
        let mut options = cap_std::fs::OpenOptions::new();
        options.read(true).follow(cap_fs_ext::FollowSymlinks::No);
        let mut file = self.dir.open_with(path.as_path(), &options).map_err(MoltenError::from)?;
        if !file.metadata().map_err(MoltenError::from)?.is_file() {
            return Err(MoltenError::invalid_harness(format!(
                "local store read leaf {} changed away from a regular file",
                path.display()
            )));
        }
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).map_err(MoltenError::from)?;
        Ok(bytes)
    }

    pub fn read_to_string(&self, path: &Locator) -> Result<String> {
        String::from_utf8(self.read(path)?).map_err(|error| {
            MoltenError::invalid_harness(format!("local store file {} is not UTF-8: {error}", path.display()))
        })
    }

    pub fn write(&self, path: &Locator, contents: &[u8]) -> Result<()> {
        if let Some(parent) = path.as_path().parent()
            && !parent.as_os_str().is_empty()
        {
            let parent_path = Locator {
                relative: parent.to_path_buf(),
            };
            self.create_dir_all(&parent_path)?;
        }
        match self.entry_kind_optional(path)? {
            Some(MemberKind::File) | None => {}
            Some(kind) => {
                return Err(MoltenError::invalid_harness(format!(
                    "local store write leaf {} must be a regular file, got {kind:?}",
                    path.display()
                )));
            }
        }
        let mut options = cap_std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true).follow(cap_fs_ext::FollowSymlinks::No);
        let mut file = self.dir.open_with(path.as_path(), &options).map_err(MoltenError::from)?;
        if !file.metadata().map_err(MoltenError::from)?.is_file() {
            return Err(MoltenError::invalid_harness(format!(
                "local store write leaf {} changed away from a regular file",
                path.display()
            )));
        }
        file.write_all(contents).map_err(MoltenError::from)?;
        file.flush().map_err(MoltenError::from)
    }

    pub fn remove_file(&self, path: &Locator) -> Result<()> {
        self.dir.remove_file(path.as_path()).map_err(MoltenError::from)
    }

    pub fn remove_dir_all(&self, path: &Locator) -> Result<()> {
        self.dir.remove_dir_all(path.as_path()).map_err(MoltenError::from)
    }

    pub fn try_exists(&self, path: &Locator) -> Result<bool> {
        self.entry_kind_optional(path).map(|kind| kind.is_some())
    }

    pub fn entry_kind(&self, path: &Locator) -> Result<MemberKind> {
        self.entry_kind_optional(path)?
            .ok_or_else(|| MoltenError::invalid_harness(format!("local store path {} does not exist", path.display())))
    }

    pub fn entry_kind_optional(&self, path: &Locator) -> Result<Option<MemberKind>> {
        match self.dir.symlink_metadata(path.as_path()) {
            Ok(metadata) => Ok(Some(entry_kind(&metadata.file_type()))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(MoltenError::from(error)),
        }
    }

    pub fn list_entries(&self, path: &Locator) -> Result<Vec<Member>> {
        let mut entries = Vec::new();
        for entry_result in self.dir.read_dir(path.as_path()).map_err(MoltenError::from)? {
            let entry = entry_result.map_err(MoltenError::from)?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let entry_path = path.join(&name)?;
            let kind = entry_kind(&entry.file_type().map_err(MoltenError::from)?);
            if entries.len() >= MAX_LOCAL_STORE_ENTRIES {
                return Err(entry_limit_error());
            }
            entries.push(Member {
                name,
                path: entry_path,
                kind,
            });
        }
        entries.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(entries)
    }

    pub fn list_file_names(&self, path: &Locator) -> Result<Vec<String>> {
        let entries = self.list_entries(path)?;
        let mut names = Vec::new();
        for entry in entries {
            if entry.kind == MemberKind::File {
                if names.len() >= MAX_LOCAL_STORE_ENTRIES {
                    return Err(entry_limit_error());
                }
                names.push(entry.name);
            }
        }
        Ok(names)
    }

    pub fn open_database_file(&self, path: &Locator) -> Result<std::fs::File> {
        match self.dir.symlink_metadata(path.as_path()) {
            Ok(metadata) => {
                let kind = entry_kind(&metadata.file_type());
                if kind != MemberKind::File {
                    return Err(MoltenError::invalid_harness(format!(
                        "database leaf {} must be a regular file, got {kind:?}",
                        path.display()
                    )));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(MoltenError::from(error)),
        }

        if let Some(parent) = path.as_path().parent()
            && !parent.as_os_str().is_empty()
        {
            let parent_path = Locator {
                relative: parent.to_path_buf(),
            };
            self.create_dir_all(&parent_path)?;
        }

        let mut options = cap_std::fs::OpenOptions::new();
        options.read(true).write(true).create(true).follow(cap_fs_ext::FollowSymlinks::No);
        let file = self.dir.open_with(path.as_path(), &options).map_err(MoltenError::from)?;
        if !file.metadata().map_err(MoltenError::from)?.is_file() {
            return Err(MoltenError::invalid_harness(format!(
                "database leaf {} must remain a regular file after open",
                path.display()
            )));
        }
        Ok(file.into_std())
    }
}

macro_rules! typed_root {
    ($name:ident, $kind:expr) => {
        pub struct $name {
            root: StorageRoot,
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.debug_tuple(stringify!($name)).field(&self.root.kind()).finish()
            }
        }

        impl $name {
            pub fn open(path: &Path) -> Result<Self> {
                Ok(Self {
                    root: StorageRoot::open($kind, path)?,
                })
            }

            pub fn open_existing(path: &Path) -> Result<Self> {
                Ok(Self {
                    root: StorageRoot::open_existing($kind, path)?,
                })
            }

            pub fn root(&self) -> &StorageRoot {
                &self.root
            }
        }
    };
}

typed_root!(ArtifactStoreRoot, Kind::Artifact);
typed_root!(ChunkStoreRoot, Kind::Chunk);
typed_root!(RetentionStoreRoot, Kind::Retention);
typed_root!(DataspaceStoreRoot, Kind::Dataspace);
typed_root!(ExchangeStoreRoot, Kind::Exchange);
typed_root!(LedgerStoreRoot, Kind::Ledger);
typed_root!(DeliveryStoreRoot, Kind::Delivery);
typed_root!(DurableStoreRoot, Kind::Durable);

impl ArtifactStoreRoot {
    pub(crate) fn from_dir(dir: cap_std::fs::Dir) -> Self {
        Self {
            root: StorageRoot::from_dir(Kind::Artifact, dir),
        }
    }
}

impl ChunkStoreRoot {
    pub(crate) fn from_dir(dir: cap_std::fs::Dir) -> Self {
        Self {
            root: StorageRoot::from_dir(Kind::Chunk, dir),
        }
    }
}

impl LedgerStoreRoot {
    pub(crate) fn from_dir(dir: cap_std::fs::Dir) -> Self {
        Self {
            root: StorageRoot::from_dir(Kind::Ledger, dir),
        }
    }
}

impl DeliveryStoreRoot {
    pub(crate) fn from_dir(dir: cap_std::fs::Dir) -> Self {
        Self {
            root: StorageRoot::from_dir(Kind::Delivery, dir),
        }
    }
}

impl ChunkStoreRoot {
    #[doc(hidden)]
    pub fn open_artifact_chunks(parent: &ArtifactStoreRoot) -> Result<Self> {
        Ok(Self {
            root: parent.root().open_subdir(Kind::Chunk, &Locator::parse("chunks")?)?,
        })
    }
}
