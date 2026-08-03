//! `udf skills {install,list,remove}` — minimal cross-provider skill
//! distribution for the 4 providers we target. **Each install location is
//! verified against the provider's official docs** (sources below):
//!
//! | Provider         | Reads `<base>/...`                          | Source                                           |
//! |------------------|--------------------------------------------|--------------------------------------------------|
//! | opencode         | `.opencode/`, `.claude/`, `.agents/` (proj + global) | sst/opencode `skills.mdx`              |
//! | codex            | `.codex/skills/`, `.agents/skills/`        | Codex App / developers.openai.com/codex/skills     |
//! | claude code      | `.claude/`                                  | anthropics/claude-code plugins/README.md         |
//! | github copilot   | (not supported — no skills concept)        | github/copilot-cli README only mentions LSP      |
//!
//! Design (simplest possible):
//! - The source of truth is `<repo>/skills/unrealdevflow/SKILL.md`.
//! - `install` copies the source into project (`./.codex/`, `./.agents/`,
//!   `./.claude/`, `./.opencode/`) and/or global (`~/.codex/`, `~/.agents/`,
//!   `~/.claude/`, `~/.config/opencode/skills/`) locations.
//! - `list` reports presence/absence of each (project + global × 4 dirs).
//! - `remove` deletes the 4 dirs (project or global).
//!
//! No junctions, no hash tracking, no auto-sync. The user is the version source:
//! `git pull` the UnrealDevFlow repo → re-run `udf skills install`.

use crate::error::{Result, UdfError};
use crate::output;
use std::fs;
use std::path::{Path, PathBuf};

pub const SKILL_NAME: &str = "unrealdevflow";
const SKILL_FILE: &str = "SKILL.md";

/// Where this binary expects to find the source `SKILL.md` next to it.
fn source_skill_file() -> Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let exe_dir = exe
        .parent()
        .ok_or_else(|| UdfError::Other(format!("无法获取 exe 目录：{:?}", exe)))?;

    // Try several candidate locations (relative to the compiled binary).
    // - dev:  target/release/udf.exe → ../../../skills/unrealdevflow/SKILL.md
    // - dev:  target/release/udf.exe → ../../skills/.../SKILL.md
    // - dev:  target/release/udf.exe → skills/.../SKILL.md
    // - dev:  target/debug/...
    // - install (e.g. ~/.cargo/bin/udf.exe): ../../skills/.../SKILL.md
    let candidates: [PathBuf; 6] = [
        exe_dir.join("../../../skills/unrealdevflow/SKILL.md"),
        exe_dir.join("../../skills/unrealdevflow/SKILL.md"),
        exe_dir.join("../skills/unrealdevflow/SKILL.md"),
        exe_dir.join("skills/unrealdevflow/SKILL.md"),
        exe_dir
            .join("../../../skills")
            .join(SKILL_NAME)
            .join(SKILL_FILE),
        exe_dir
            .join("../../skills")
            .join(SKILL_NAME)
            .join(SKILL_FILE),
    ];

    for c in &candidates {
        if c.exists() {
            return Ok(c.clone());
        }
    }

    // Also try `cargo run` / `cargo build` mode (CWD == workspace root).
    if let Ok(cwd) = std::env::current_dir() {
        let cargo_path = cwd.join("skills/unrealdevflow/SKILL.md");
        if cargo_path.exists() {
            return Ok(cargo_path);
        }
    }

    // Helpful debug: show what we searched and what the current exe is.
    let searched = candidates
        .iter()
        .map(|c| {
            format!(
                "  - {:?} ({})",
                c,
                if c.exists() { "exists" } else { "missing" }
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    Err(UdfError::Other(format!(
        "找不到源 SKILL.md。\n可执行: {:?}\n搜索过的路径:\n{}\n提示：在 dev 模式下请 `cargo run`；在 release 模式下 SKILL.md 必须位于可执行文件旁。",
        exe, searched
    )))
}

/// Project-scope install locations. 4 paths for 3 supported providers
/// (copilot has no skill support so it's intentionally absent).
fn target_paths_for_project(base: &Path) -> [(PathBuf, &'static str); 4] {
    [
        (
            base.join(".codex").join("skills").join(SKILL_NAME),
            "codex (native)",
        ),
        (
            base.join(".agents").join("skills").join(SKILL_NAME),
            "codex/opencode (agents fallback)",
        ),
        (
            base.join(".claude").join("skills").join(SKILL_NAME),
            "claude code",
        ),
        (
            base.join(".opencode").join("skills").join(SKILL_NAME),
            "opencode (native)",
        ),
    ]
}

/// Global-scope install locations.
///
/// **Note** the opencode path differs from project: `~/.config/opencode/` (XDG)
/// not `~/.opencode/`. This is per opencode's official docs (skills.mdx).
fn target_paths_for_global(base: &Path) -> [(PathBuf, &'static str); 4] {
    [
        (
            base.join(".codex").join("skills").join(SKILL_NAME),
            "codex (native)",
        ),
        (
            base.join(".agents").join("skills").join(SKILL_NAME),
            "codex/opencode (agents fallback)",
        ),
        (
            base.join(".claude").join("skills").join(SKILL_NAME),
            "claude code",
        ),
        (
            base.join(".config")
                .join("opencode")
                .join("skills")
                .join(SKILL_NAME),
            "opencode (native, XDG path)",
        ),
    ]
}

/// One provider directory the skill was written to.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InstalledLocation {
    provider: String,
    path: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillInstalled {
    scope: String,
    source: String,
    locations: Vec<InstalledLocation>,
}

fn install_to_base(base: &Path, source: &Path, scope: &str) -> Result<Vec<InstalledLocation>> {
    let content = fs::read_to_string(source)
        .map_err(|e| UdfError::Other(format!("读取源 SKILL.md 失败：{}", e)))?;

    let targets = match scope {
        "project" => target_paths_for_project(base).to_vec(),
        "global" => target_paths_for_global(base).to_vec(),
        _ => return Err(UdfError::Other(format!("内部错误：未知 scope '{}'", scope))),
    };

    let mut installed = Vec::new();
    for (dir, label) in &targets {
        fs::create_dir_all(dir)?;
        let dest = dir.join(SKILL_FILE);
        fs::write(&dest, &content)?;
        installed.push(InstalledLocation {
            provider: (*label).to_string(),
            path: dest.to_string_lossy().to_string(),
        });
    }
    Ok(installed)
}

pub fn install(global: bool, project: Option<PathBuf>) -> Result<()> {
    let installed = install_inner(global, project)?;
    output::emit("skill install", installed, render_installed);
    Ok(())
}

/// The copy itself, without emitting. `workspace init` runs it as one stage of
/// a larger command, so only the outermost command answers.
pub(crate) fn install_inner(global: bool, project: Option<PathBuf>) -> Result<SkillInstalled> {
    let source = source_skill_file()?;

    let (scope, base) = if global {
        let home =
            dirs::home_dir().ok_or_else(|| UdfError::Other("无法确定用户主目录".to_string()))?;
        ("global", home)
    } else {
        let cwd = match project {
            Some(p) => p,
            None => std::env::current_dir()?,
        };
        ("project", cwd)
    };
    let locations = install_to_base(&base, &source, scope)?;

    Ok(SkillInstalled {
        scope: scope.to_string(),
        source: source.to_string_lossy().to_string(),
        locations,
    })
}

fn render_installed(data: &SkillInstalled) -> String {
    let mut lines = vec![format!("Source SKILL.md: {}", data.source)];
    for location in &data.locations {
        lines.push(format!(
            "  [{}] {} → {}",
            data.scope, location.provider, location.path
        ));
    }
    lines.push(format!(
        "✓ Installed {} location(s) for scope '{}'.",
        data.locations.len(),
        data.scope
    ));
    lines.join("\n")
}

pub fn list(project: Option<PathBuf>) -> Result<()> {
    let cwd = match project {
        Some(p) => p,
        None => std::env::current_dir()?,
    };
    let home = dirs::home_dir().ok_or_else(|| UdfError::Other("无法确定用户主目录".to_string()))?;

    let scopes = [("project", cwd), ("global", home)];

    let mut entries = Vec::new();
    for (scope, base) in &scopes {
        let targets = match *scope {
            "project" => target_paths_for_project(base).to_vec(),
            "global" => target_paths_for_global(base).to_vec(),
            _ => Vec::new(),
        };
        for (dir, label) in &targets {
            let skill_file = dir.join(SKILL_FILE);
            let present = skill_file.exists();
            entries.push(SkillLocation {
                scope: (*scope).to_string(),
                provider: (*label).to_string(),
                path: skill_file.to_string_lossy().to_string(),
                installed: present,
                size_bytes: if present {
                    fs::metadata(&skill_file).ok().map(|meta| meta.len())
                } else {
                    None
                },
            });
        }
    }

    let installed = entries.iter().filter(|entry| entry.installed).count();
    let total = entries.len();
    output::emit(
        "skill list",
        SkillStatus {
            installed,
            total,
            locations: entries,
        },
        render_status,
    );
    Ok(())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SkillLocation {
    scope: String,
    provider: String,
    path: String,
    installed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    size_bytes: Option<u64>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SkillStatus {
    installed: usize,
    total: usize,
    locations: Vec<SkillLocation>,
}

fn render_status(data: &SkillStatus) -> String {
    let rule = "─".repeat(61);
    let mut lines = vec![
        String::new(),
        "ℹ UnrealDevFlow skill installation status".to_string(),
        rule.clone(),
    ];
    let mut scope = "";
    for location in &data.locations {
        if location.scope != scope {
            scope = &location.scope;
            lines.push(String::new());
            lines.push(format!("ℹ Scope: {}", scope));
        }
        if location.installed {
            lines.push(format!(
                "✓   {:<38} {}  ({} bytes)",
                location.provider,
                location.path,
                location.size_bytes.unwrap_or(0)
            ));
        } else {
            lines.push(format!(
                "⚠   ✗ {:<38} not found: {}",
                location.provider, location.path
            ));
        }
    }
    lines.push(String::new());
    lines.push(rule);
    lines.push(format!(
        "ℹ Coverage: {}/{} location(s) installed",
        data.installed, data.total
    ));
    if data.installed < data.total {
        lines.push("ℹ To install: udf skill install [--global]".to_string());
    }
    lines.join("\n")
}

pub fn remove(global: bool, project: Option<PathBuf>) -> Result<()> {
    let scopes: Vec<(&str, PathBuf)> = if global {
        let home =
            dirs::home_dir().ok_or_else(|| UdfError::Other("无法确定用户主目录".to_string()))?;
        vec![("global", home)]
    } else {
        let cwd = match project {
            Some(p) => p,
            None => std::env::current_dir()?,
        };
        vec![("project", cwd)]
    };

    let mut removed = Vec::new();
    let mut skipped = Vec::new();
    for (scope, base) in &scopes {
        let targets = match *scope {
            "project" => target_paths_for_project(base).to_vec(),
            "global" => target_paths_for_global(base).to_vec(),
            _ => Vec::new(),
        };
        for (dir, label) in &targets {
            let location = InstalledLocation {
                provider: (*label).to_string(),
                path: dir.to_string_lossy().to_string(),
            };
            if dir.exists() {
                fs::remove_dir_all(dir)?;
                removed.push(location);
            } else {
                skipped.push(location);
            }
        }
    }

    output::emit(
        "skill remove",
        SkillRemoved { removed, skipped },
        render_removed,
    );
    Ok(())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SkillRemoved {
    removed: Vec<InstalledLocation>,
    skipped: Vec<InstalledLocation>,
}

fn render_removed(data: &SkillRemoved) -> String {
    let mut lines = Vec::new();
    for location in &data.removed {
        lines.push(format!(
            "✓   removed {} ({})",
            location.provider, location.path
        ));
    }
    for location in &data.skipped {
        lines.push(format!(
            "ℹ   {} not installed (skip): {}",
            location.provider, location.path
        ));
    }
    lines.push(format!("ℹ Removed {} location(s).", data.removed.len()));
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_targets_include_codex_native_skill_dir() {
        let base = Path::new(r"C:\project");

        let targets = target_paths_for_project(base);

        assert_eq!(targets.len(), 4);
        assert!(targets.iter().any(|(path, label)| {
            path == &base.join(".codex").join("skills").join(SKILL_NAME)
                && *label == "codex (native)"
        }));
        assert!(
            targets
                .iter()
                .any(|(path, _)| { path == &base.join(".agents").join("skills").join(SKILL_NAME) })
        );
    }

    #[test]
    fn global_targets_include_codex_native_skill_dir() {
        let base = Path::new(r"C:\Users\tester");

        let targets = target_paths_for_global(base);

        assert_eq!(targets.len(), 4);
        assert!(targets.iter().any(|(path, label)| {
            path == &base.join(".codex").join("skills").join(SKILL_NAME)
                && *label == "codex (native)"
        }));
        assert!(
            targets
                .iter()
                .any(|(path, _)| { path == &base.join(".agents").join("skills").join(SKILL_NAME) })
        );
    }
}
