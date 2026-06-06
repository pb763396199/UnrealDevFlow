//! Configure command implementation

use crate::config;
use crate::error::{Result, UdfError};
use crate::output;
use std::path::PathBuf;

pub fn run(
    hosts_root: Option<String>,
    plugin_path: Option<String>,
    default_project: Option<String>,
    engine_path: Option<String>,
) -> Result<()> {
    // Check if running in non-interactive mode (all required params provided)
    let non_interactive = hosts_root.is_some() && plugin_path.is_some() && default_project.is_some();

    if non_interactive {
        // Non-interactive mode: use provided values
        let hosts_root = PathBuf::from(hosts_root.unwrap());
        let plugin_path = PathBuf::from(plugin_path.unwrap());
        let default_project = PathBuf::from(default_project.unwrap());

        // Auto-detect engine path if not provided
        let engine_path = match engine_path {
            Some(path) => PathBuf::from(path),
            None => config::detect_engine_path(&default_project)?,
        };

        let config = config::Config {
            hosts_root,
            plugin_path,
            default_project,
            engine_path,
        };

        config.save()?;

        output::print_success("Configuration saved (non-interactive mode)");
        output::print_info(&format!("  Hosts root: {:?}", config.hosts_root));
        output::print_info(&format!("  Plugin path: {:?}", config.plugin_path));
        output::print_info(&format!("  Default project: {:?}", config.default_project));
        output::print_info(&format!("  Engine path: {:?}", config.engine_path));
    } else {
        // Interactive mode
        let config = config::run_configure()?;
        output::print_success(&format!("Configuration saved to {:?}", config::Config::config_path()?));
        output::print_info(&format!("  Hosts root: {:?}", config.hosts_root));
        output::print_info(&format!("  Plugin path: {:?}", config.plugin_path));
        output::print_info(&format!("  Default project: {:?}", config.default_project));
        output::print_info(&format!("  Engine path: {:?}", config.engine_path));
    }

    Ok(())
}
