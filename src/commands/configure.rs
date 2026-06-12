//! Configure command implementation

use crate::config;
use crate::error::{Result, UdfError};
use crate::output;
use std::collections::HashMap;
use std::path::PathBuf;

pub fn run(
    hosts_root: Option<String>,
    plugin_path: Option<String>,
    plugins_root: Option<String>,
    default_project: Option<String>,
    engine_path: Option<String>,
) -> Result<()> {
    // v2 non-interactive: hosts_root + (plugins_root or plugin_path) + default_project
    let has_plugin_source = plugins_root.is_some() || plugin_path.is_some();
    let non_interactive = hosts_root.is_some() && has_plugin_source && default_project.is_some();

    if non_interactive {
        let hosts_root = PathBuf::from(hosts_root.unwrap());
        let plugin_path_opt = plugin_path.map(PathBuf::from);
        let plugins_root_opt = plugins_root.map(PathBuf::from);
        let default_project = PathBuf::from(default_project.unwrap());

        let engine_path = match engine_path {
            Some(path) => PathBuf::from(path),
            None => config::detect_engine_path(&default_project)?,
        };

        output::print_info("验证配置...");

        if !hosts_root.exists() {
            output::print_info(&format!("  创建 Hosts 目录：{:?}", hosts_root));
            std::fs::create_dir_all(&hosts_root)?;
        }

        if let Some(plugin_path) = &plugin_path_opt {
            if !plugin_path.exists() {
                return Err(UdfError::Other(format!(
                    "v1 plugin_path 不存在：{:?}\n请检查路径，或改用 --plugins-root",
                    plugin_path
                )));
            }
        }

        if let Some(plugins_root) = &plugins_root_opt {
            if !plugins_root.exists() {
                return Err(UdfError::Other(format!(
                    "plugins_root 不存在：{:?}",
                    plugins_root
                )));
            }
        }

        if !default_project.exists() {
            return Err(UdfError::Other(format!(
                "UE 项目不存在：{:?}\n请检查路径是否正确",
                default_project
            )));
        }
        let uproject_exists = std::fs::read_dir(&default_project)
            .map(|entries| {
                entries.filter_map(|e| e.ok()).any(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext == "uproject")
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);
        if !uproject_exists {
            return Err(UdfError::Other(format!(
                "UE 项目目录中没有 .uproject 文件：{:?}",
                default_project
            )));
        }

        if !engine_path.exists() {
            return Err(UdfError::Other(format!(
                "引擎路径不存在：{:?}",
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
                "引擎 Build.bat 不存在：{:?}",
                build_bat
            )));
        }

        output::print_success("配置验证通过");

        let config = config::Config {
            hosts_root,
            plugin_path: plugin_path_opt,
            default_project,
            engine_path,
            plugins_root: plugins_root_opt,
            plugin_overrides: HashMap::new(),
        };

        config.save()?;

        output::print_success("配置已保存（非交互模式）");
        output::print_info(&format!("  Hosts 根目录：{:?}", config.hosts_root));
        if let Some(pr) = &config.plugins_root {
            output::print_info(&format!("  插件根目录：{:?}", pr));
        }
        if let Some(pp) = &config.plugin_path {
            output::print_info(&format!("  v1 兼容插件路径：{:?}", pp));
        }
        output::print_info(&format!("  默认 UE 项目：{:?}", config.default_project));
        output::print_info(&format!("  引擎路径：{:?}", config.engine_path));
    } else {
        let config = config::run_configure()?;
        output::print_success(&format!(
            "配置已保存到 {:?}",
            config::Config::config_path()?
        ));
        output::print_info(&format!("  Hosts 根目录：{:?}", config.hosts_root));
        if let Some(pr) = &config.plugins_root {
            output::print_info(&format!("  插件根目录：{:?}", pr));
        }
        output::print_info(&format!("  默认 UE 项目：{:?}", config.default_project));
        output::print_info(&format!("  引擎路径：{:?}", config.engine_path));
    }

    Ok(())
}
