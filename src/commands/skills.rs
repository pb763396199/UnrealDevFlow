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

fn install_to_base(base: &Path, source: &Path, scope: &str) -> Result<()> {
    let content = fs::read_to_string(source)
        .map_err(|e| UdfError::Other(format!("读取源 SKILL.md 失败：{}", e)))?;

    let targets = match scope {
        "project" => target_paths_for_project(base).to_vec(),
        "global" => target_paths_for_global(base).to_vec(),
        _ => return Err(UdfError::Other(format!("内部错误：未知 scope '{}'", scope))),
    };

    let mut installed = 0usize;
    for (dir, label) in &targets {
        fs::create_dir_all(dir)?;
        let dest = dir.join(SKILL_FILE);
        fs::write(&dest, &content)?;
        output::print_success(&format!("  [{}] {} → {:?}", scope, label, dest));
        installed += 1;
    }
    output::print_info(&format!(
        "Installed {} location(s) for scope '{}'.",
        installed, scope
    ));
    Ok(())
}

pub fn install(global: bool, project: Option<PathBuf>) -> Result<()> {
    let source = source_skill_file()?;
    output::print_info(&format!("Source SKILL.md: {:?}", source));

    if global {
        let home =
            dirs::home_dir().ok_or_else(|| UdfError::Other("无法确定用户主目录".to_string()))?;
        install_to_base(&home, &source, "global")?;
    } else {
        let cwd = match project {
            Some(p) => p,
            None => std::env::current_dir()?,
        };
        install_to_base(&cwd, &source, "project")?;
    }
    Ok(())
}

pub fn list(project: Option<PathBuf>) -> Result<()> {
    let cwd = match project {
        Some(p) => p,
        None => std::env::current_dir()?,
    };
    let home = dirs::home_dir().ok_or_else(|| UdfError::Other("无法确定用户主目录".to_string()))?;

    let scopes = [("project", cwd), ("global", home)];

    println!();
    output::print_info("UnrealDevFlow skill installation status");
    println!("─────────────────────────────────────────────────────────────");

    let mut total_present = 0;
    let mut total = 0;
    for (scope, base) in &scopes {
        println!();
        output::print_info(&format!("Scope: {} (base: {:?})", scope, base));
        let targets = match *scope {
            "project" => target_paths_for_project(base).to_vec(),
            "global" => target_paths_for_global(base).to_vec(),
            _ => Vec::new(),
        };
        for (dir, label) in &targets {
            total += 1;
            let skill_file = dir.join(SKILL_FILE);
            if skill_file.exists() {
                total_present += 1;
                let meta = fs::metadata(&skill_file).ok();
                let size = meta.map(|m| m.len()).unwrap_or(0);
                output::print_success(&format!(
                    "  ✓ {:<38} {:?}  ({} bytes)",
                    label, skill_file, size
                ));
            } else {
                output::print_warning(&format!("  ✗ {:<38} not found: {:?}", label, dir));
            }
        }
    }

    println!();
    println!("─────────────────────────────────────────────────────────────");
    output::print_info(&format!(
        "Coverage: {}/{} location(s) installed",
        total_present, total
    ));

    if total_present < total {
        output::print_info("To install: udf skills install [--global]");
    }

    Ok(())
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

    let mut removed = 0usize;
    for (scope, base) in &scopes {
        let targets = match *scope {
            "project" => target_paths_for_project(base).to_vec(),
            "global" => target_paths_for_global(base).to_vec(),
            _ => Vec::new(),
        };
        for (dir, label) in &targets {
            if dir.exists() {
                fs::remove_dir_all(dir)?;
                output::print_success(&format!("  [{}] removed {} ({:?})", scope, label, dir));
                removed += 1;
            } else {
                output::print_info(&format!(
                    "  [{}] {} not installed (skip): {:?}",
                    scope, label, dir
                ));
            }
        }
    }
    output::print_info(&format!("Removed {} location(s).", removed));
    Ok(())
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
