//! Junction validation

use crate::error::Result;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum JunctionState {
    Missing,
    NotAJunction,
    Valid(PathBuf),
    Broken(PathBuf),
    InvalidTarget(PathBuf),
}

pub fn validate(path: &Path) -> Result<JunctionState> {
    if !path.exists() {
        return Ok(JunctionState::Missing);
    }

    if !super::exists(path)? {
        return Ok(JunctionState::NotAJunction);
    }

    match super::get_target(path) {
        Ok(target) => {
            if !target.exists() {
                Ok(JunctionState::Broken(target))
            } else if !target.is_dir() {
                Ok(JunctionState::InvalidTarget(target))
            } else {
                Ok(JunctionState::Valid(target))
            }
        }
        Err(_) => Ok(JunctionState::NotAJunction),
    }
}
