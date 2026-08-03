//! Junction management module

pub mod validator;

use crate::error::{JunctionError, Result};
use crate::output;
use std::path::{Path, PathBuf};

/// Is this path a junction, whatever its target?
///
/// The `junction` crate resolves the target, so a junction whose target was
/// deleted answers "no" while the directory entry is still on disk. Every
/// broken-junction recovery path in this tool depends on the answer being
/// "yes", so we fall back to asking the filesystem about the entry itself.
pub fn exists(path: &Path) -> Result<bool> {
    match junction::exists(path) {
        Ok(true) => Ok(true),
        Ok(false) => Ok(is_reparse_point(path)),
        Err(error) => Err(JunctionError::JunctionCrate(error.to_string()).into()),
    }
}

/// A directory entry that redirects elsewhere: junction, symlink or mount point.
fn is_reparse_point(path: &Path) -> bool {
    std::fs::symlink_metadata(path)
        .map(|meta| meta.file_type().is_symlink())
        .unwrap_or(false)
}

pub fn create(target: &Path, junction: &Path) -> Result<()> {
    if !target.exists() {
        return Err(JunctionError::TargetNotFound(target.to_path_buf()).into());
    }

    // Ensure parent directory exists
    if let Some(parent) = junction.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // junction::create will create the directory itself
    junction::create(target, junction)
        .map_err(|e| JunctionError::JunctionCrate(e.to_string()).into())
}

pub fn delete(junction_path: &Path) -> Result<()> {
    if !exists(junction_path)? {
        return Err(JunctionError::NotExists(junction_path.to_path_buf()).into());
    }

    // A dangling junction makes the crate's delete fail, but removing the
    // directory entry works either way. On Windows `remove_dir` on a reparse
    // point removes the link and never touches the target.
    if junction::delete(junction_path).is_err() {
        std::fs::remove_dir(junction_path)?;
        return Ok(());
    }

    if is_reparse_point(junction_path) || junction_path.exists() {
        std::fs::remove_dir(junction_path)?;
    }

    Ok(())
}

pub fn get_target(junction: &Path) -> Result<PathBuf> {
    junction::get_target(junction).map_err(|e| JunctionError::JunctionCrate(e.to_string()).into())
}

/// Check if a junction is "broken" (target directory no longer exists).
/// Returns true if:
/// - The path is a valid junction
/// - But the target directory does not exist
pub fn is_broken(junction: &Path) -> bool {
    if !exists(junction).unwrap_or(false) {
        return false; // Not a junction at all
    }

    match get_target(junction) {
        Ok(target) => !target.exists(),
        Err(_) => true, // Can't read target, treat as broken
    }
}

pub fn switch(target: &Path, junction: &Path) -> Result<()> {
    // Ask about the entry, not what it resolves to: a dangling junction still
    // occupies the name.
    if is_reparse_point(junction) || junction.exists() {
        if exists(junction)? {
            // It's a junction, delete it
            delete(junction)?;
        } else {
            // It's a regular directory, backup it
            let backup_path = PathBuf::from(format!("{}.udf-backup", junction.to_string_lossy()));
            if backup_path.exists() {
                // Backup already exists, remove it first
                std::fs::remove_dir_all(&backup_path)?;
            }
            output::print_info(&format!(
                "Backing up existing directory to {:?}",
                backup_path
            ));
            std::fs::rename(junction, &backup_path)?;
        }
    }
    create(target, junction)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    /// A junction whose target was deleted still occupies the name. Every
    /// broken-junction recovery path depends on us noticing that.
    #[test]
    fn a_dangling_junction_is_still_reported_and_can_be_deleted() {
        let root = tempdir().expect("temp dir");
        let target = root.path().join("Target");
        let link = root.path().join("Link");
        std::fs::create_dir_all(&target).expect("target");
        create(&target, &link).expect("create junction");

        assert!(exists(&link).expect("exists"));
        assert!(!is_broken(&link), "target is still there");

        std::fs::remove_dir_all(&target).expect("remove target");

        assert!(!link.exists(), "Path::exists follows the reparse point");
        assert!(exists(&link).expect("exists"), "the entry is still on disk");
        assert!(is_broken(&link), "and it is now broken");

        delete(&link).expect("a dangling junction must still be removable");
        assert!(std::fs::symlink_metadata(&link).is_err(), "entry is gone");
    }

    /// Deleting a junction must never touch what it points at.
    #[test]
    fn deleting_a_live_junction_leaves_the_target_alone() {
        let root = tempdir().expect("temp dir");
        let target = root.path().join("Target");
        let link = root.path().join("Link");
        std::fs::create_dir_all(&target).expect("target");
        std::fs::write(target.join("keep.txt"), "keep me").expect("payload");
        create(&target, &link).expect("create junction");

        delete(&link).expect("delete junction");

        assert!(std::fs::symlink_metadata(&link).is_err(), "link is gone");
        assert!(target.join("keep.txt").is_file(), "target survived");
    }
}
