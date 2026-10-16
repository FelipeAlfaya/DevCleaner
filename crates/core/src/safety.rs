use std::path::{Component, Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SafetyError {
    #[error("invalid path")]
    InvalidPath,
    #[error("path outside the allowed root")]
    OutsideJail,
    #[error("protected system path")]
    ProtectedSystemPath,
    #[error("symlink or junction not allowed")]
    SymlinkEscape,
}

impl SafetyError {
    pub fn msg(&self) -> crate::i18n::Msg {
        use crate::i18n::Msg;
        match self {
            SafetyError::InvalidPath => Msg::SafetyInvalidPath,
            SafetyError::OutsideJail => Msg::SafetyOutsideJail,
            SafetyError::ProtectedSystemPath => Msg::SafetyProtected,
            SafetyError::SymlinkEscape => Msg::SafetySymlink,
        }
    }
}

pub fn validate_delete_path(jail_root: &Path, candidate: &Path) -> Result<PathBuf, SafetyError> {
    let jail = canonicalize_existing_or_self(jail_root)?;
    let resolved = resolve_under_jail(&jail, candidate)?;

    if is_protected_system_path(&resolved) {
        return Err(SafetyError::ProtectedSystemPath);
    }

    if !resolved.starts_with(&jail) {
        return Err(SafetyError::OutsideJail);
    }

    Ok(resolved)
}

fn canonicalize_existing_or_self(path: &Path) -> Result<PathBuf, SafetyError> {
    if path.exists() {
        std::fs::canonicalize(path).map_err(|_| SafetyError::InvalidPath)
    } else if let Some(parent) = path.parent() {
        if parent.as_os_str().is_empty() {
            return Ok(path.to_path_buf());
        }
        let mut canon_parent = std::fs::canonicalize(parent).map_err(|_| SafetyError::InvalidPath)?;
        if let Some(name) = path.file_name() {
            canon_parent.push(name);
        }
        Ok(canon_parent)
    } else {
        Ok(path.to_path_buf())
    }
}

fn resolve_under_jail(jail: &Path, candidate: &Path) -> Result<PathBuf, SafetyError> {
    if candidate.is_absolute() {
        if candidate.exists() {
            let meta = std::fs::symlink_metadata(candidate).map_err(|_| SafetyError::InvalidPath)?;
            if meta.file_type().is_symlink() {
                return Err(SafetyError::SymlinkEscape);
            }
            return std::fs::canonicalize(candidate).map_err(|_| SafetyError::InvalidPath);
        }
        return Ok(candidate.to_path_buf());
    }

    let mut acc = jail.to_path_buf();
    for component in candidate.components() {
        match component {
            Component::ParentDir => {
                if !acc.pop() {
                    return Err(SafetyError::OutsideJail);
                }
            }
            Component::Normal(c) => acc.push(c),
            Component::CurDir => {}
            Component::RootDir | Component::Prefix(_) => return Err(SafetyError::OutsideJail),
        }
    }
    Ok(acc)
}

fn is_protected_system_path(path: &Path) -> bool {
    let s = path.to_string_lossy().to_lowercase();
    let markers = [
        "\\windows\\system32",
        "\\program files",
        "\\program files (x86)",
        "/usr/bin",
        "/usr/sbin",
        "/bin",
        "/sbin",
        "/etc",
        "/sys",
        "/proc",
        "\\appdata\\local\\docker",
    ];
    markers.iter().any(|m| s.contains(m))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn allows_path_inside_jail() {
        let dir = tempdir().unwrap();
        let jail = dir.path().join("cache");
        fs::create_dir_all(&jail).unwrap();
        let file = jail.join("pkg.tgz");
        fs::write(&file, b"x").unwrap();

        let resolved = validate_delete_path(&jail, &file).unwrap();
        assert!(resolved.starts_with(std::fs::canonicalize(&jail).unwrap()));
    }

    #[test]
    fn rejects_parent_traversal() {
        let dir = tempdir().unwrap();
        let jail = dir.path().join("cache");
        fs::create_dir_all(&jail).unwrap();
        let outside = dir.path().join("secret.txt");
        fs::write(&outside, b"x").unwrap();

        let attempt = jail.join("..").join("secret.txt");
        assert_eq!(
            validate_delete_path(&jail, &attempt),
            Err(SafetyError::OutsideJail)
        );
    }
}
