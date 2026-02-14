//! Utility methods for parsing raw git objects.

use aspen_core::KeyValueStore;

use super::super::error::BridgeError;
use super::super::error::BridgeResult;
use super::super::mapping::GitObjectType;
use super::GitObjectConverter;

impl<K: KeyValueStore + ?Sized> GitObjectConverter<K> {
    /// Get the object type from git object bytes.
    pub fn parse_git_object_type(git_bytes: &[u8]) -> BridgeResult<(&str, &[u8])> {
        // Find space after type
        let space_pos = git_bytes.iter().position(|&b| b == b' ').ok_or_else(|| BridgeError::MalformedObject {
            message: "missing space in git object header".to_string(),
        })?;

        let type_str = std::str::from_utf8(&git_bytes[..space_pos])?;

        // Find NUL after size
        let nul_pos = git_bytes.iter().position(|&b| b == 0).ok_or_else(|| BridgeError::MalformedObject {
            message: "missing NUL in git object header".to_string(),
        })?;

        let content = &git_bytes[nul_pos + 1..];

        Ok((type_str, content))
    }

    /// Parse a full git object (with header) and return type and content.
    pub fn split_git_object(git_bytes: &[u8]) -> BridgeResult<(GitObjectType, &[u8])> {
        let (type_str, content) = Self::parse_git_object_type(git_bytes)?;

        let obj_type = GitObjectType::parse(type_str).ok_or_else(|| BridgeError::UnknownObjectType {
            type_str: type_str.to_string(),
        })?;

        Ok((obj_type, content))
    }
}
