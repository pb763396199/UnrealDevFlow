//! Plugin discovery, classification, and management module.
//!
//! Provides the data model and APIs used by the v2 multi-plugin support:
//! - `scanner` walks engine and project plugin roots and classifies plugins.
//! - `uplugin` parses `.uplugin` JSON files to extract their dependency lists.

pub mod scanner;
pub mod uplugin;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginSource {
    Engine,
    Project,
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredPlugin {
    pub name: String,
    pub source: PluginSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone, Default)]
pub struct DependencyResolution {
    pub engine: Vec<DiscoveredPlugin>,
    pub project: Vec<DiscoveredPlugin>,
    pub conflict: Vec<(DiscoveredPlugin, DiscoveredPlugin)>,
    pub missing: Vec<String>,
}
