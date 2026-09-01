use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const DEFAULT_PACKAGE_ARGS: &[&str] = &[
    "-nocompileeditor",
    "-nop4",
    "-pak",
    "-cook",
    "-stage",
    "-archive",
    "-package",
    "-compressed",
    "-prereqs",
    "-build",
    "-utf8output",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UePlatform {
    Windows,
    Linux,
    WindowsWithLinux,
}

impl UePlatform {
    pub fn uat_target_platform(self) -> &'static str {
        match self {
            UePlatform::Windows => "Win64",
            UePlatform::Linux | UePlatform::WindowsWithLinux => "Linux",
        }
    }

    fn installed_build_linux_enabled(self) -> bool {
        !matches!(self, UePlatform::Windows)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Configuration {
    Development,
    Shipping,
    Debug,
    Test,
    Custom(String),
}

impl Configuration {
    #[allow(dead_code)]
    pub fn parse(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "development" => Self::Development,
            "shipping" => Self::Shipping,
            "debug" => Self::Debug,
            "test" => Self::Test,
            _ => Self::Custom(value.to_string()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageContainer {
    Loose,
    Pak,
    Iostore,
}

impl Configuration {
    pub fn as_unreal_value(&self) -> &str {
        match self {
            Configuration::Development => "Development",
            Configuration::Shipping => "Shipping",
            Configuration::Debug => "Debug",
            Configuration::Test => "Test",
            Configuration::Custom(value) => value,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UbtMutexMode {
    NoMutex,
    Wait,
    FailFast,
}

impl UbtMutexMode {
    pub fn ubt_arg(self) -> Option<&'static str> {
        match self {
            UbtMutexMode::NoMutex => Some("-NoMutex"),
            UbtMutexMode::Wait => Some("-WaitMutex"),
            UbtMutexMode::FailFast => None,
        }
    }

    fn uat_ubtargs(self) -> Option<String> {
        self.ubt_arg().map(|arg| format!("-ubtargs={arg}"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UeCommand {
    pub executable: String,
    pub arguments: Vec<String>,
}

impl UeCommand {
    pub fn new(executable: impl Into<String>, arguments: impl IntoIterator<Item = String>) -> Self {
        Self {
            executable: executable.into(),
            arguments: arguments.into_iter().collect(),
        }
    }

    pub fn argv(&self) -> Vec<String> {
        let mut argv = Vec::with_capacity(self.arguments.len() + 1);
        argv.push(self.executable.clone());
        argv.extend(self.arguments.iter().cloned());
        argv
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectPackageOptions {
    pub engine_root: PathBuf,
    pub project: PathBuf,
    pub archive_dir: PathBuf,
    pub platform: UePlatform,
    pub configuration: Configuration,
    pub mutex: UbtMutexMode,
    pub package_args: Option<Vec<String>>,
    pub container: PackageContainer,
    pub clean: bool,
    pub native_settings: Option<NativePackageSettings>,
    pub iterate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativePackageSettings {
    pub build: Option<String>,
    pub full_rebuild: Option<bool>,
    pub include_debug_files: Option<bool>,
    pub cook_all: Option<bool>,
    pub cook_maps_only: Option<bool>,
    pub skip_editor_content: Option<bool>,
    pub compressed: Option<bool>,
    pub include_prerequisites: Option<bool>,
    pub use_zen_store: Option<bool>,
    pub maps_to_cook: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineSourceBuildOptions {
    pub engine_root: PathBuf,
    pub platform: UePlatform,
    pub configuration: Configuration,
    pub mutex: UbtMutexMode,
    pub gitdeps_threads: usize,
    pub gitdeps_cache: Option<PathBuf>,
    pub editor_target: String,
    pub extra_targets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledBuildOptions {
    pub engine_root: PathBuf,
    pub output_dir: PathBuf,
    pub platform: UePlatform,
}

pub fn project_package_commands(options: &ProjectPackageOptions) -> Vec<UeCommand> {
    let package_args = resolved_package_args(options);
    let target = options
        .project
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("Game");
    let unreal_exe = options
        .engine_root
        .join("Engine/Binaries/Win64/UnrealEditor-Cmd.exe");
    let mut arguments = vec![
        "BuildCookRun".to_string(),
        format!("-project={}", path_arg(&options.project)),
        format!("-target={target}"),
        format!("-archivedirectory={}", path_arg(&options.archive_dir)),
        format!("-targetplatform={}", options.platform.uat_target_platform()),
        format!("-unrealexe={}", path_arg(&unreal_exe)),
        "-installed".to_string(),
        "-skipbuildeditor".to_string(),
        "-nocompile".to_string(),
        "-nocompileuat".to_string(),
    ];
    arguments.extend(package_args);

    if !has_uat_ubtargs(&arguments)
        && let Some(ubtargs) = options.mutex.uat_ubtargs()
    {
        arguments.push(ubtargs);
    }
    if options.clean {
        arguments.push("-Clean".to_string());
    }

    vec![UeCommand::new(runuat_path(&options.engine_root), arguments)]
}

pub fn engine_source_build_commands(options: &EngineSourceBuildOptions) -> Vec<UeCommand> {
    let mut gitdeps_args = vec![
        format!("--threads={}", options.gitdeps_threads),
        "--force".to_string(),
    ];
    if let Some(cache) = &options.gitdeps_cache {
        gitdeps_args.push(format!("--cache={}", path_arg(cache)));
    }

    let target_platform = options.platform.uat_target_platform();
    let configuration = options.configuration.as_unreal_value();
    let mut build_args = vec![
        format!(
            "-Target={} {} {} -Quiet",
            options.editor_target, target_platform, configuration
        ),
        format!(
            "-Target=ShaderCompileWorker {} {} -Quiet",
            target_platform, configuration
        ),
    ];
    build_args.extend(options.extra_targets.iter().map(|target| {
        format!(
            "-Target={} {} {} -Quiet",
            target, target_platform, configuration
        )
    }));
    if let Some(mutex_arg) = options.mutex.ubt_arg() {
        build_args.push(mutex_arg.to_string());
    }
    build_args.push("-FromMsBuild".to_string());

    vec![
        UeCommand::new(gitdependencies_path(&options.engine_root), gitdeps_args),
        UeCommand::new(
            generate_project_files_path(&options.engine_root),
            Vec::new(),
        ),
        UeCommand::new(build_bat_path(&options.engine_root), build_args),
    ]
}

pub fn installed_build_commands(options: &InstalledBuildOptions) -> Vec<UeCommand> {
    let with_linux = options.platform.installed_build_linux_enabled();
    vec![UeCommand::new(
        runuat_path(&options.engine_root),
        vec![
            "BuildGraph".to_string(),
            "-target=Make Installed Build Win64".to_string(),
            "-script=Engine/Build/InstalledEngineBuild.xml".to_string(),
            "-set:WithMac=false".to_string(),
            "-set:WithTVOS=false".to_string(),
            format!("-set:WithLinux={}", bool_arg(with_linux)),
            format!("-set:WithLinuxArm64={}", bool_arg(with_linux)),
            "-set:WithAndroid=false".to_string(),
            "-set:WithIOS=false".to_string(),
            format!("-set:BuiltDirectory={}", path_arg(&options.output_dir)),
        ],
    )]
}

fn resolved_package_args(options: &ProjectPackageOptions) -> Vec<String> {
    let package_args = options
        .package_args
        .clone()
        .unwrap_or_else(|| native_package_args(options));
    let mut package_args = package_args;
    if matches!(options.container, PackageContainer::Loose) {
        package_args.retain(|arg| {
            !arg.eq_ignore_ascii_case("-pak") && !arg.eq_ignore_ascii_case("-iostore")
        });
    } else if matches!(options.container, PackageContainer::Iostore)
        && !package_args
            .iter()
            .any(|arg| arg.eq_ignore_ascii_case("-iostore"))
    {
        package_args.push("-iostore".to_string());
    }
    if has_client_config(&package_args) {
        package_args
    } else {
        let mut resolved = Vec::with_capacity(package_args.len() + 1);
        resolved.push(format!(
            "-clientconfig={}",
            options.configuration.as_unreal_value()
        ));
        resolved.extend(package_args);
        resolved
    }
}

fn native_package_args(options: &ProjectPackageOptions) -> Vec<String> {
    let Some(settings) = options.native_settings.as_ref() else {
        return DEFAULT_PACKAGE_ARGS
            .iter()
            .map(|arg| (*arg).to_string())
            .collect();
    };
    let mut args = vec!["-nocompileeditor".to_string(), "-nop4".to_string()];
    match settings.build.as_deref() {
        Some(value) if value.eq_ignore_ascii_case("never") => args.push("-skipbuild".into()),
        _ => args.push("-build".into()),
    }
    if settings.full_rebuild == Some(true) {
        args.push("-clean".into());
    }
    if settings.include_debug_files == Some(false) {
        args.push("-nodebuginfo".into());
    }
    args.push("-cook".into());
    if options.iterate {
        args.push("-iterate".into());
    }
    if settings.cook_all == Some(true) {
        args.push("-allmaps".into());
        if settings.cook_maps_only == Some(true) {
            args.push("-cookmapsonly".into());
        }
    }
    if settings.skip_editor_content == Some(true) {
        args.push("-skipcookingeditorcontent".into());
    }
    args.extend(["-stage".into(), "-archive".into(), "-package".into()]);
    match options.container {
        PackageContainer::Loose => {}
        PackageContainer::Pak => args.push("-pak".into()),
        PackageContainer::Iostore => args.push("-iostore".into()),
    }
    if settings.compressed == Some(true) {
        args.push("-compressed".into());
    }
    if settings.include_prerequisites == Some(true) {
        args.push("-prereqs".into());
    }
    if settings.use_zen_store == Some(true) {
        args.push("-zenstore".into());
    }
    if !settings.maps_to_cook.is_empty() {
        args.push(format!("-mapstocook={}", settings.maps_to_cook.join("+")));
    }
    args.push("-utf8output".into());
    args
}

fn has_client_config(args: &[String]) -> bool {
    args.iter().any(|arg| arg.starts_with("-clientconfig="))
}

fn has_uat_ubtargs(args: &[String]) -> bool {
    args.iter()
        .any(|arg| arg.to_ascii_lowercase().starts_with("-ubtargs"))
}

fn runuat_path(engine_root: &Path) -> String {
    path_arg(
        engine_root
            .join("Engine")
            .join("Build")
            .join("BatchFiles")
            .join("RunUAT.bat"),
    )
}

fn build_bat_path(engine_root: &Path) -> String {
    path_arg(
        engine_root
            .join("Engine")
            .join("Build")
            .join("BatchFiles")
            .join("Build.bat"),
    )
}

fn generate_project_files_path(engine_root: &Path) -> String {
    path_arg(engine_root.join("GenerateProjectFiles.bat"))
}

fn gitdependencies_path(engine_root: &Path) -> String {
    path_arg(
        engine_root
            .join("Engine")
            .join("Binaries")
            .join("DotNET")
            .join("GitDependencies")
            .join("win-x64")
            .join("GitDependencies.exe"),
    )
}

fn path_arg(path: impl AsRef<Path>) -> String {
    path.as_ref().to_string_lossy().replace('\\', "/")
}

fn bool_arg(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}
