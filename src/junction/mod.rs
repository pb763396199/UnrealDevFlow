//! Junction management module

pub mod validator;

use crate::error::{JunctionError, Result, UdfError};
use crate::output;
use std::path::{Path, PathBuf};

pub fn exists(path: &Path) -> Result<bool> {
    junction::exists(path).map_err(|e| JunctionError::JunctionCrate(e.to_string()).into())
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
    junction::create(target, junction).map_err(|e| JunctionError::JunctionCrate(e.to_string()).into())
}

pub fn delete(junction: &Path) -> Result<()> {
    if !exists(junction)? {
        return Err(JunctionError::NotExists(junction.to_path_buf()).into());
    }

    junction::delete(junction).map_err(|e| -> UdfError { JunctionError::JunctionCrate(e.to_string()).into() })?;

    // Remove the empty directory
    if junction.exists() {
        std::fs::remove_dir(junction)?;
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
    // Check if junction path exists
    if junction.exists() {
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
            output::print_info(&format!("Backing up existing directory to {:?}", backup_path));
            std::fs::rename(junction, &backup_path)?;
        }
    }
    create(target, junction)
}
