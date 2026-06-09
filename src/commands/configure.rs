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

        // === 配置验证 ===
        output::print_info("验证配置...");

        // 1. 验证 Hosts 根目录
        if !hosts_root.exists() {
            output::print_info(&format!("  创建 Hosts 目录：{:?}", hosts_root));
            std::fs::create_dir_all(&hosts_root)?;
        }

        // 2. 验证插件主仓库
        if !plugin_path.exists() {
            return Err(UdfError::Other(format!(
                "插件主仓库不存在：{:?}\n请检查路径是否正确，或先克隆仓库",
                plugin_path
            )));
        }
        if !plugin_path.join(".git").exists() {
            return Err(UdfError::Other(format!(
                "插件主仓库不是 Git 仓库：{:?}\n请确保路径包含 .git 目录",
                plugin_path
            )));
        }

        // 3. 验证 UE 项目
        if !default_project.exists() {
            return Err(UdfError::Other(format!(
                "UE 项目不存在：{:?}\n请检查路径是否正确",
                default_project
            )));
        }
        let uproject_exists = std::fs::read_dir(&default_project)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .any(|e| {
                        e.path()
                            .extension()
                            .map(|ext| ext == "uproject")
                            .unwrap_or(false)
                    })
            })
            .unwrap_or(false);
        if !uproject_exists {
            return Err(UdfError::Other(format!(
                "UE 项目目录中没有 .uproject 文件：{:?}\n请检查路径是否正确",
                default_project
            )));
        }

        // 4. 验证引擎路径
        if !engine_path.exists() {
            return Err(UdfError::Other(format!(
                "引擎路径不存在：{:?}\n请检查引擎是否已安装",
                engine_path
            )));
        }
        let build_bat = engine_path
            .join("Engine")
            .join("Build")
            .join("BatchFiles")
            .join("Build.bat");
        if !build_bat.exists() {
            return Err(UdfError::Other(format!(
                "引擎 Build.bat 不存在：{:?}\n请检查引擎安装是否完整",
                build_bat
            )));
        }

        output::print_success("配置验证通过");

        let config = config::Config {
            hosts_root,
            plugin_path,
            default_project,
            engine_path,
        };

        config.save()?;

        output::print_success("配置已保存（非交互模式）");
        output::print_info(&format!("  Hosts 根目录：{:?}", config.hosts_root));
        output::print_info(&format!("  插件主仓库：{:?}", config.plugin_path));
        output::print_info(&format!("  默认 UE 项目：{:?}", config.default_project));
        output::print_info(&format!("  引擎路径：{:?}", config.engine_path));
    } else {
        // Interactive mode
        let config = config::run_configure()?;
        output::print_success(&format!("配置已保存到 {:?}", config::Config::config_path()?));
        output::print_info(&format!("  Hosts 根目录：{:?}", config.hosts_root));
        output::print_info(&format!("  插件主仓库：{:?}", config.plugin_path));
        output::print_info(&format!("  默认 UE 项目：{:?}", config.default_project));
        output::print_info(&format!("  引擎路径：{:?}", config.engine_path));
    }

    Ok(())
}
