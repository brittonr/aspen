use std::{
    fs::File,
    io::{BufRead, BufReader},
    num::ParseIntError,
    path::PathBuf,
};

use nix::{
    errno::Errno,
    unistd::{Gid, Group, Uid, User},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum SubordinateError {
    #[error("can't determine user {0}")]
    UidError(Errno),

    #[error("user entry for {0} does not exist")]
    NoPasswdEntry(Uid),

    #[error("can't determine group {0}")]
    GidError(Errno),

    #[error("group entry for {0} does not exist")]
    NoGroupEntry(Gid),

    #[error("io error {0:?}, file {1}")]
    IoError(std::io::Error, PathBuf),

    #[error("failed to parse {0} line '{1}', error {2}")]
    ParseError(PathBuf, String, ParseIntError),

    #[error("Missing entry in {0}, for {1}({2})")]
    MissingEntry(PathBuf, String, u32),
}

/// Represents a single (subuid,subgid) pair for a user and their group.
///
/// In practice there are usually many more subordinate ids than just one, but
/// for oci builds we only need one. If we ever need more, we can improve this
/// implementation.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct SubordinateInfo {
    pub uid: u32,
    pub gid: u32,
    pub subuid: u32,
    pub subgid: u32,
}

impl SubordinateInfo {
    /// Parses /etc/subuid and /etc/subgid and returns a single [SubordinateInfo] for the effective user.
    pub(crate) fn for_effective_user() -> Result<SubordinateInfo, SubordinateError> {
        let (user, group) = user_info()?;

        let subuid =
            first_subordinate_id(&PathBuf::from("/etc/subuid"), user.uid.as_raw(), &user.name)?;
        let subgid =
            first_subordinate_id(&PathBuf::from("/etc/subgid"), user.uid.as_raw(), &user.name)?;
        Ok(SubordinateInfo {
            uid: user.uid.as_raw(),
            gid: group.gid.as_raw(),
            subuid,
            subgid,
        })
    }
}

/// Returns user and group entries for current effective user.
fn user_info() -> Result<(User, Group), SubordinateError> {
    let u = Uid::effective();
    let user = User::from_uid(u)
        .map_err(SubordinateError::UidError)?
        .ok_or(SubordinateError::NoPasswdEntry(u))?;
    let g = Gid::effective();
    let group = Group::from_gid(g)
        .map_err(SubordinateError::GidError)?
        .ok_or(SubordinateError::NoGroupEntry(g))?;
    Ok((user, group))
}

fn first_subordinate_id(file: &PathBuf, id: u32, name: &str) -> Result<u32, SubordinateError> {
    let f = File::open(file).map_err(|e| SubordinateError::IoError(e, file.clone()))?;
    let reader = BufReader::new(f).lines();

    for line in reader {
        let line = line.map_err(|e| SubordinateError::IoError(e, file.clone()))?;
        let line = line.trim();
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() == 3 && (parts[0] == name || id.to_string() == parts[0]) {
            let subuid = parts[1]
                .parse::<u32>()
                .map_err(|e| SubordinateError::ParseError(file.clone(), line.into(), e))?;
            let range = parts[2]
                .parse::<u32>()
                .map_err(|e| SubordinateError::ParseError(file.clone(), line.into(), e))?;
            if range > 0 {
                return Ok(subuid);
            }
        }
    }

    Err(SubordinateError::MissingEntry(
        file.clone(),
        name.into(),
        id,
    ))
}

#[cfg(test)]
mod tests {
    use crate::oci::subuid::SubordinateError;

    fn create_fixture<'a>(content: impl IntoIterator<Item = &'a str>) -> tempfile::NamedTempFile {
        use std::io::Write;
        let mut file = tempfile::NamedTempFile::new().expect("Could not create tempfile");
        for line in content.into_iter() {
            writeln!(file, "{line}").expect("");
        }
        file
    }

    #[test]
    fn test_parse_uid_file_with_name_should_return_first_match() {
        let file = create_fixture(["nobody:10000:65", "root:1000:2", "0:2:2"]);
        let id = super::first_subordinate_id(&file.path().into(), 0, "root")
            .expect("Faild to look up subordinate id.");
        assert_eq!(id, 1000);
    }

    #[test]
    fn test_parse_uid_file_with_uid_should_return_first_match() {
        let file = create_fixture(["nobody:10000:65", "0:2:2"]);
        let id = super::first_subordinate_id(&file.path().into(), 0, "root")
            .expect("Failed to look up subordinate id.");
        assert_eq!(id, 2);
    }

    #[test]
    fn test_missing() {
        let file = create_fixture(["roots:1000:2", "1000:2:2"]);
        let id = super::first_subordinate_id(&file.path().into(), 0, "root")
            .expect_err("Expected not to find a matching subordinate entry.");
        assert!(matches!(id, SubordinateError::MissingEntry(_, _, _)));
    }

    #[test]
    fn test_parse_error() {
        let file = create_fixture(["root:hello:2", "1000:2:2"]);
        let id = super::first_subordinate_id(&file.path().into(), 0, "root")
            .expect_err("Expected parsing to fail.");
        assert!(matches!(id, SubordinateError::ParseError(_, _, _)));
    }

    #[test]
    fn test_parse_errors_in_other_users_files_are_ignored() {
        let file = create_fixture(["root:hello:2", "1000:2:2"]);
        let id = super::first_subordinate_id(&file.path().into(), 1000, "user")
            .expect("Failed to look up subordinate id.");
        assert_eq!(id, 2);
    }
}
