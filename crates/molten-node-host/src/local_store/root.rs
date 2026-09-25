use std::io::Read;
use std::io::Write;

use cap_fs_ext::OpenOptionsFollowExt;

type Failure = crate::error::Failure;
type Result<T> = crate::error::Result<T>;
type Path = std::path::Path;

const MAX_LOCAL_STORE_ENTRIES: usize = 100_000;

const _: () = assert!(MAX_LOCAL_STORE_ENTRIES <= 1_000_000);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectKind {
    File,
    Directory,
    Symlink,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredEntry {
    pub name: String,
    pub path: super::path::RelativeLocator,
    pub kind: ObjectKind,
}

pub struct DirectoryHandle {
    kind: super::path::Category,
    dir: cap_std::fs::Dir,
}

impl std::fmt::Debug for DirectoryHandle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("DirectoryHandle").field("kind", &self.kind).finish_non_exhaustive()
    }
}

impl DirectoryHandle {
    pub fn open(kind: super::path::Category, root: &Path) -> Result<Self> {
        std::fs::create_dir_all(root).map_err(Failure::from)?;
        Self::open_existing(kind, root)
    }

    pub fn open_existing(kind: super::path::Category, root: &Path) -> Result<Self> {
        let dir = cap_std::fs::Dir::open_ambient_dir(root, cap_std::ambient_authority()).map_err(Failure::from)?;
        Ok(Self { kind, dir })
    }

    pub fn kind(&self) -> super::path::Category {
        self.kind
    }

    #[doc(hidden)]
    pub fn try_clone_dir(&self) -> Result<cap_std::fs::Dir> {
        self.dir.try_clone().map_err(Failure::from)
    }

    pub(crate) fn from_dir(kind: super::path::Category, dir: cap_std::fs::Dir) -> Self {
        Self { kind, dir }
    }

    pub(super) fn open_subdir(&self, kind: super::path::Category, path: &super::path::RelativeLocator) -> Result<Self> {
        self.create_dir_all(path)?;
        let dir = self.dir.open_dir(path.as_path()).map_err(Failure::from)?;
        Ok(Self { kind, dir })
    }

    pub(super) fn share_authority_as(&self, kind: super::path::Category) -> Result<Self> {
        let dir = self.dir.try_clone().map_err(Failure::from)?;
        Ok(Self { kind, dir })
    }

    pub fn create_dir_all(&self, path: &super::path::RelativeLocator) -> Result<()> {
        self.dir.create_dir_all(path.as_path()).map_err(Failure::from)
    }

    pub fn read(&self, path: &super::path::RelativeLocator) -> Result<Vec<u8>> {
        if self.entry_kind(path)? != ObjectKind::File {
            return Err(Failure::invalid_harness(format!(
                "local store read leaf {} must be a regular file",
                path.display()
            )));
        }
        let mut options = cap_std::fs::OpenOptions::new();
        options.read(true).follow(cap_fs_ext::FollowSymlinks::No);
        let mut file = self.dir.open_with(path.as_path(), &options).map_err(Failure::from)?;
        if !file.metadata().map_err(Failure::from)?.is_file() {
            return Err(Failure::invalid_harness(format!(
                "local store read leaf {} changed away from a regular file",
                path.display()
            )));
        }
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).map_err(Failure::from)?;
        Ok(bytes)
    }

    pub fn read_to_string(&self, path: &super::path::RelativeLocator) -> Result<String> {
        String::from_utf8(self.read(path)?).map_err(|error| {
            Failure::invalid_harness(format!("local store file {} is not UTF-8: {error}", path.display()))
        })
    }

    pub fn write(&self, path: &super::path::RelativeLocator, contents: &[u8]) -> Result<()> {
        if let Some(parent) = path.as_path().parent()
            && !parent.as_os_str().is_empty()
        {
            let parent_path = super::path::RelativeLocator {
                relative: parent.to_path_buf(),
            };
            self.create_dir_all(&parent_path)?;
        }
        match self.entry_kind_optional(path)? {
            Some(ObjectKind::File) | None => {}
            Some(kind) => {
                return Err(Failure::invalid_harness(format!(
                    "local store write leaf {} must be a regular file, got {kind:?}",
                    path.display()
                )));
            }
        }
        let mut options = cap_std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true).follow(cap_fs_ext::FollowSymlinks::No);
        let mut file = self.dir.open_with(path.as_path(), &options).map_err(Failure::from)?;
        if !file.metadata().map_err(Failure::from)?.is_file() {
            return Err(Failure::invalid_harness(format!(
                "local store write leaf {} changed away from a regular file",
                path.display()
            )));
        }
        file.write_all(contents).map_err(Failure::from)?;
        file.flush().map_err(Failure::from)
    }

    pub fn remove_file(&self, path: &super::path::RelativeLocator) -> Result<()> {
        self.dir.remove_file(path.as_path()).map_err(Failure::from)
    }

    pub fn remove_dir_all(&self, path: &super::path::RelativeLocator) -> Result<()> {
        self.dir.remove_dir_all(path.as_path()).map_err(Failure::from)
    }

    pub fn try_exists(&self, path: &super::path::RelativeLocator) -> Result<bool> {
        self.entry_kind_optional(path).map(|kind| kind.is_some())
    }

    pub fn entry_kind(&self, path: &super::path::RelativeLocator) -> Result<ObjectKind> {
        self.entry_kind_optional(path)?
            .ok_or_else(|| Failure::invalid_harness(format!("local store path {} does not exist", path.display())))
    }

    pub fn entry_kind_optional(&self, path: &super::path::RelativeLocator) -> Result<Option<ObjectKind>> {
        match self.dir.symlink_metadata(path.as_path()) {
            Ok(metadata) => Ok(Some(entry_kind(&metadata.file_type()))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(Failure::from(error)),
        }
    }

    pub fn list_entries(&self, path: &super::path::RelativeLocator) -> Result<Vec<StoredEntry>> {
        let mut entries = Vec::new();
        for entry_result in self.dir.read_dir(path.as_path()).map_err(Failure::from)? {
            let entry = entry_result.map_err(Failure::from)?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let entry_path = path.join(&name)?;
            let kind = entry_kind(&entry.file_type().map_err(Failure::from)?);
            push_bounded_entry(
                &mut entries,
                StoredEntry {
                    name,
                    path: entry_path,
                    kind,
                },
            )?;
        }
        entries.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(entries)
    }

    pub fn list_file_names(&self, path: &super::path::RelativeLocator) -> Result<Vec<String>> {
        let entries = self.list_entries(path)?;
        let mut names = Vec::new();
        for entry in entries {
            if entry.kind == ObjectKind::File {
                push_bounded_name(&mut names, entry.name)?;
            }
        }
        Ok(names)
    }

    pub fn open_database_file(&self, path: &super::path::RelativeLocator) -> Result<std::fs::File> {
        match self.dir.symlink_metadata(path.as_path()) {
            Ok(metadata) => {
                let kind = entry_kind(&metadata.file_type());
                if kind != ObjectKind::File {
                    return Err(Failure::invalid_harness(format!(
                        "database leaf {} must be a regular file, got {kind:?}",
                        path.display()
                    )));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(Failure::from(error)),
        }

        if let Some(parent) = path.as_path().parent()
            && !parent.as_os_str().is_empty()
        {
            let parent_path = super::path::RelativeLocator {
                relative: parent.to_path_buf(),
            };
            self.create_dir_all(&parent_path)?;
        }

        let mut options = cap_std::fs::OpenOptions::new();
        options.read(true).write(true).create(true).follow(cap_fs_ext::FollowSymlinks::No);
        let file = self.dir.open_with(path.as_path(), &options).map_err(Failure::from)?;
        if !file.metadata().map_err(Failure::from)?.is_file() {
            return Err(Failure::invalid_harness(format!(
                "database leaf {} must remain a regular file after open",
                path.display()
            )));
        }
        Ok(file.into_std())
    }
}

fn entry_kind(file_type: &cap_std::fs::FileType) -> ObjectKind {
    if file_type.is_file() {
        ObjectKind::File
    } else if file_type.is_dir() {
        ObjectKind::Directory
    } else if file_type.is_symlink() {
        ObjectKind::Symlink
    } else {
        ObjectKind::Other
    }
}

fn push_bounded_entry(entries: &mut Vec<StoredEntry>, entry: StoredEntry) -> Result<()> {
    ensure_entry_capacity(entries.len())?;
    entries.push(entry);
    Ok(())
}

fn push_bounded_name(names: &mut Vec<String>, name: String) -> Result<()> {
    ensure_entry_capacity(names.len())?;
    names.push(name);
    Ok(())
}

fn ensure_entry_capacity(current: usize) -> Result<()> {
    let next = current.checked_add(1).ok_or_else(|| Failure::invalid_harness("local store entry count overflow"))?;
    if next > MAX_LOCAL_STORE_ENTRIES {
        return Err(Failure::invalid_harness(format!(
            "local store entry count {next} exceeds maximum {MAX_LOCAL_STORE_ENTRIES}"
        )));
    }
    Ok(())
}
