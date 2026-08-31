use std::path::PathBuf;

use crate::error::Result;

pub fn configure(
    workspace: Option<String>,
    task: Option<String>,
    configuration: Option<String>,
    container: Option<String>,
    output: Option<PathBuf>,
    disable_plugin: Vec<String>,
    file: Option<PathBuf>,
    reason: Option<String>,
) -> Result<()> {
    crate::package_profile::configure(
        workspace,
        task,
        configuration,
        container,
        output,
        disable_plugin,
        file,
        reason,
    )
}
