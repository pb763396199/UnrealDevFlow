//! Configure command implementation

use crate::config;
use crate::error::Result;
use crate::output;

pub fn run() -> Result<()> {
    let config = config::run_configure()?;
    output::print_success(&format!("Configuration saved to {:?}", config::Config::config_path()?));
    output::print_info(&format!("  Hosts root: {:?}", config.hosts_root));
    output::print_info(&format!("  Plugin path: {:?}", config.plugin_path));
    if let Some(project) = &config.default_project {
        output::print_info(&format!("  Default project: {:?}", project));
    }
    if let Some(engine) = &config.engine_path {
        output::print_info(&format!("  Engine path: {:?}", engine));
    }
    Ok(())
}
