//! Junction management module

pub mod validator;

use crate::error::{JunctionError, Result, UdfError};
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

    // Create empty directory for junction
    std::fs::create_dir_all(junction)?;

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

pub fn switch(target: &Path, junction: &Path) -> Result<()> {
    if exists(junction)? {
        delete(junction)?;
    }
    create(target, junction)
}
