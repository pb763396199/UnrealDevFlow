//! .uproject file generation

pub fn generate(engine_version: &str) -> String {
    format!(
        r#"{{"FileVersion":3,"EngineAssociation":"{}","Modules":[],"Plugins":[{{"Name":"AesWorld","Enabled":true}}]}}"#,
        engine_version
    )
}

pub fn detect_engine_version(uproject_path: &std::path::Path) -> Option<String> {
    let content = std::fs::read_to_string(uproject_path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    json["EngineAssociation"].as_str().map(|s| s.to_string())
}
