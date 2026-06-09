//! .uproject file generation

use std::path::PathBuf;

/// Generate a Host `.uproject` enabling the given plugin names.
///
/// All entries become `{"Name": <name>, "Enabled": true}` in the `Plugins`
/// array. Engine-provided plugins (e.g. `GeometryProcessing`) just need to be
/// listed here to be picked up by UBT for compilation.
pub fn generate(engine_version: &str, enabled_plugins: &[String]) -> String {
    let plugins_json = enabled_plugins
        .iter()
        .map(|name| format!(r#"{{"Name":"{}","Enabled":true}}"#, name))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#"{{"FileVersion":3,"EngineAssociation":"{}","Modules":[],"Plugins":[{}]}}"#,
        engine_version, plugins_json
    )
}

pub fn detect_engine_version(uproject_path: &std::path::Path) -> Option<String> {
    let content = std::fs::read_to_string(uproject_path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    json["EngineAssociation"].as_str().map(|s| s.to_string())
}

/// Convenience: regenerate a Host's `.uproject` for the given enabled plugin
/// list, preserving the existing engine version.
pub fn rewrite(uproject_path: &PathBuf, enabled_plugins: &[String]) -> std::io::Result<()> {
    let engine_version = detect_engine_version(uproject_path).unwrap_or_else(|| "5.5".to_string());
    let content = generate(&engine_version, enabled_plugins);
    std::fs::write(uproject_path, content)
}
