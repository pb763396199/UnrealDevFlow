//! Project-authored Unreal packaging settings used as the UDF baseline.
//!
//! UE remains the final configuration interpreter. This module only reads the
//! project-owned settings that affect the generated Package Project command and
//! records their source and digest for reproducibility.

use crate::error::{Result, UdfError};
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageContainer {
    Loose,
    #[default]
    Pak,
    Iostore,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackagingSettings {
    pub build: Option<String>,
    pub configuration: Option<String>,
    pub build_target: Option<String>,
    pub full_rebuild: Option<bool>,
    pub include_debug_files: Option<bool>,
    pub container: PackageContainer,
    pub compressed: Option<bool>,
    pub compression_format: Option<String>,
    pub compression_method: Option<String>,
    pub include_prerequisites: Option<bool>,
    pub use_zen_store: Option<bool>,
    pub cook_all: Option<bool>,
    pub cook_maps_only: Option<bool>,
    pub skip_editor_content: Option<bool>,
    pub skip_movies: Option<bool>,
    pub always_cook: Vec<String>,
    pub never_cook: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CookerSettings {
    pub iterative_cooking_for_file_cook_content: Option<bool>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapSettings {
    pub game_default_map: Option<String>,
    pub maps_to_cook: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectPackagingSnapshot {
    pub digest: String,
    pub source_files: Vec<PathBuf>,
    #[serde(default)]
    pub source_digests: Vec<String>,
    pub packaging: PackagingSettings,
    pub cooker: CookerSettings,
    pub maps: MapSettings,
}

#[derive(Default)]
struct IniSection {
    scalars: BTreeMap<String, String>,
    arrays: BTreeMap<String, Vec<String>>,
}

#[derive(Default)]
struct IniDocument {
    sections: BTreeMap<String, IniSection>,
}

pub fn load_snapshot(project_root: &Path) -> Result<ProjectPackagingSnapshot> {
    let config_root = project_root.join("Config");
    let mut files = Vec::new();
    for name in ["DefaultGame.ini", "DefaultEngine.ini"] {
        let path = config_root.join(name);
        if path.is_file() {
            files.push(path);
        }
    }
    let windows_root = config_root.join("Windows");
    if windows_root.is_dir() {
        let mut platform_files = fs::read_dir(&windows_root)?
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("ini"))
            })
            .collect::<Vec<_>>();
        platform_files.sort();
        files.extend(platform_files);
    }

    let mut document = IniDocument::default();
    for path in &files {
        parse_ini_file(path, &mut document)?;
    }

    let packaging = parse_packaging(&document);
    let cooker = parse_cooker(&document);
    let maps = parse_maps(&document);
    let mut snapshot = ProjectPackagingSnapshot {
        digest: String::new(),
        source_digests: files
            .iter()
            .map(|path| {
                fs::read(path)
                    .map(|bytes| format!("md5:{:x}", Md5::digest(bytes)))
                    .map_err(|error| {
                        UdfError::Other(format!("读取项目配置失败：{}：{}", path.display(), error))
                    })
            })
            .collect::<Result<Vec<_>>>()?,
        source_files: files,
        packaging,
        cooker,
        maps,
    };
    snapshot.digest = snapshot_digest(&snapshot)?;
    Ok(snapshot)
}

fn parse_ini_file(path: &Path, document: &mut IniDocument) -> Result<()> {
    let text = fs::read_to_string(path).map_err(|error| {
        UdfError::Other(format!("读取项目配置失败：{}：{}", path.display(), error))
    })?;
    let mut section_name = String::new();
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section_name = normalize_section(&line[1..line.len() - 1]);
            document.sections.entry(section_name.clone()).or_default();
            continue;
        }
        let Some((raw_key, raw_value)) = line.split_once('=') else {
            continue;
        };
        if section_name.is_empty() {
            continue;
        }
        let (operation, key) = match raw_key.as_bytes().first() {
            Some(b'+') => ('+', raw_key[1..].trim()),
            Some(b'-') => ('-', raw_key[1..].trim()),
            _ => ('=', raw_key.trim()),
        };
        if key.is_empty() {
            continue;
        }
        let section = document
            .sections
            .get_mut(&section_name)
            .expect("section created");
        let value = raw_value.trim().to_string();
        if operation == '=' {
            section.scalars.insert(key.to_ascii_lowercase(), value);
        } else {
            let values = section.arrays.entry(key.to_ascii_lowercase()).or_default();
            if operation == '-' {
                values.retain(|existing| existing != &value);
            } else {
                values.push(value);
            }
        }
    }
    Ok(())
}

fn normalize_section(section: &str) -> String {
    section.trim().to_ascii_lowercase()
}

fn section<'a>(document: &'a IniDocument, suffix: &str) -> Option<&'a IniSection> {
    document
        .sections
        .iter()
        .find(|(name, _)| name.ends_with(&suffix.to_ascii_lowercase()))
        .map(|(_, section)| section)
}

fn scalar(section: Option<&IniSection>, key: &str) -> Option<String> {
    section?.scalars.get(&key.to_ascii_lowercase()).cloned()
}

fn bool_value(value: Option<String>) -> Option<bool> {
    value.and_then(|value| match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" => Some(true),
        "false" | "0" => Some(false),
        _ => None,
    })
}

fn quoted_value(value: &str, key: &str) -> Option<String> {
    let marker = format!(r#"{key}="#);
    let start = value.find(&marker)? + marker.len();
    let rest = value[start..].strip_prefix('"').unwrap_or(&value[start..]);
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn array_values(section: Option<&IniSection>, key: &str, embedded_key: &str) -> Vec<String> {
    section
        .and_then(|section| section.arrays.get(&key.to_ascii_lowercase()))
        .into_iter()
        .flatten()
        .filter_map(|value| {
            quoted_value(value, embedded_key).or_else(|| {
                let value = value.trim().trim_matches('"');
                (!value.is_empty()).then(|| value.to_string())
            })
        })
        .collect()
}

fn platform_configuration(section: Option<&IniSection>) -> Option<String> {
    let value = scalar(section, "perplatformbuildconfig")?;
    let windows = value
        .split("Windows")
        .nth(1)
        .and_then(|value| value.split("PPBC_").nth(1))?;
    Some(
        windows
            .split(|ch: char| !ch.is_ascii_alphanumeric())
            .next()?
            .to_string(),
    )
}

fn configuration_name(value: Option<String>) -> Option<String> {
    value.map(|value| {
        value
            .trim()
            .trim_start_matches("PPBC_")
            .to_ascii_lowercase()
            .chars()
            .enumerate()
            .map(|(index, ch)| {
                if index == 0 {
                    ch.to_ascii_uppercase()
                } else {
                    ch
                }
            })
            .collect()
    })
}

fn parse_packaging(document: &IniDocument) -> PackagingSettings {
    let section = section(document, "projectpackagingsettings");
    let use_pak = bool_value(scalar(section, "usepakfile"));
    let use_iostore = bool_value(scalar(section, "buseiostore"));
    let container = if use_iostore == Some(true) {
        PackageContainer::Iostore
    } else if use_pak == Some(false) {
        PackageContainer::Loose
    } else {
        PackageContainer::Pak
    };
    PackagingSettings {
        build: scalar(section, "build"),
        configuration: configuration_name(
            platform_configuration(section)
                .or_else(|| scalar(section, "buildconfiguration"))
                .or_else(|| Some("Development".to_string())),
        ),
        build_target: scalar(section, "buildtarget"),
        full_rebuild: bool_value(scalar(section, "fullrebuild")),
        include_debug_files: bool_value(scalar(section, "includedebugfiles")),
        container,
        compressed: bool_value(scalar(section, "bcompressed")),
        compression_format: scalar(section, "packagecompressionformat"),
        compression_method: scalar(section, "packagecompressionmethod"),
        include_prerequisites: bool_value(scalar(section, "includeprerequisites")),
        use_zen_store: bool_value(scalar(section, "busezenstore")),
        cook_all: bool_value(scalar(section, "bcookall")),
        cook_maps_only: bool_value(scalar(section, "bcookmapsonly")),
        skip_editor_content: bool_value(scalar(section, "bskipeditorcontent")),
        skip_movies: bool_value(scalar(section, "bskipmovies")),
        always_cook: array_values(section, "directoriestoalwayscook", "Path"),
        never_cook: array_values(section, "directorystonevercook", "Path"),
    }
}

fn parse_cooker(document: &IniDocument) -> CookerSettings {
    let section = section(document, "cookersettings");
    CookerSettings {
        iterative_cooking_for_file_cook_content: bool_value(scalar(
            section,
            "iterativecookingforfilecookcontent",
        )),
    }
}

fn parse_maps(document: &IniDocument) -> MapSettings {
    let maps_section = section(document, "projectpackagingsettings");
    let game_maps_section = section(document, "gamemapssettings");
    MapSettings {
        game_default_map: scalar(game_maps_section, "gamedefaultmap"),
        maps_to_cook: array_values(maps_section, "mapstocook", "FilePath"),
    }
}

fn snapshot_digest(snapshot: &ProjectPackagingSnapshot) -> Result<String> {
    let value = (
        &snapshot.source_files,
        &snapshot.source_digests,
        &snapshot.packaging,
        &snapshot.cooker,
        &snapshot.maps,
    );
    Ok(format!(
        "md5:{:x}",
        Md5::digest(serde_json::to_vec(&value).map_err(|error| {
            UdfError::Other(format!("项目打包设置摘要失败：{error}"))
        })?)
    ))
}

#[cfg(test)]
mod tests {
    use super::{PackageContainer, configuration_name, load_snapshot, platform_configuration};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn parses_windows_platform_build_configuration() {
        let value = super::IniSection {
            scalars: [(
                "perplatformbuildconfig".into(),
                "((\"Windows\", PPBC_Shipping))".into(),
            )]
            .into_iter()
            .collect(),
            arrays: Default::default(),
        };
        assert_eq!(
            platform_configuration(Some(&value)),
            Some("Shipping".into())
        );
        assert_eq!(
            configuration_name(Some("PPBC_Shipping".into())),
            Some("Shipping".into())
        );
    }

    #[test]
    fn defaults_container_to_pak() {
        assert_eq!(PackageContainer::default(), PackageContainer::Pak);
    }

    #[test]
    fn reads_project_packaging_settings_and_windows_override() {
        let temp = TempDir::new().unwrap();
        let config = temp.path().join("Config");
        fs::create_dir_all(config.join("Windows")).unwrap();
        fs::write(
            config.join("DefaultGame.ini"),
            r#"[/Script/UnrealEd.ProjectPackagingSettings]
Build=IfProjectHasCode
BuildConfiguration=PPBC_Development
FullRebuild=False
IncludeDebugFiles=True
UsePakFile=True
bUseIoStore=False
bCompressed=True
PackageCompressionFormat=Oodle
PackageCompressionMethod=Kraken
IncludePrerequisites=True
bCookAll=False
bSkipEditorContent=False
+DirectoriesToAlwaysCook=(Path="/Game/Always")
"#,
        )
        .unwrap();
        fs::write(
            config.join("Windows").join("WindowsGame.ini"),
            r#"[/Script/UnrealEd.ProjectPackagingSettings]
PerPlatformBuildConfig=(("Windows", PPBC_Shipping))
"#,
        )
        .unwrap();
        fs::write(
            config.join("DefaultEngine.ini"),
            r#"[/Script/EngineSettings.GameMapsSettings]
GameDefaultMap=/Game/Maps/Main
"#,
        )
        .unwrap();

        let snapshot = load_snapshot(temp.path()).unwrap();
        assert_eq!(snapshot.packaging.configuration, Some("Shipping".into()));
        assert_eq!(snapshot.packaging.container, PackageContainer::Pak);
        assert_eq!(snapshot.packaging.compressed, Some(true));
        assert_eq!(snapshot.packaging.include_debug_files, Some(true));
        assert_eq!(snapshot.packaging.always_cook, vec!["/Game/Always"]);
        assert_eq!(
            snapshot.maps.game_default_map,
            Some("/Game/Maps/Main".into())
        );
        assert!(!snapshot.digest.is_empty());
    }

    #[test]
    fn missing_project_config_has_safe_native_defaults() {
        let temp = TempDir::new().unwrap();
        let snapshot = load_snapshot(temp.path()).unwrap();
        assert_eq!(snapshot.packaging.configuration, Some("Development".into()));
        assert_eq!(snapshot.packaging.container, PackageContainer::Pak);
        assert_eq!(snapshot.packaging.compressed, None);
    }
}
