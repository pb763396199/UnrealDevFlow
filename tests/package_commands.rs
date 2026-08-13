#[path = "../src/ue_commands.rs"]
mod ue_commands;

use std::path::PathBuf;

use ue_commands::{
    Configuration, EngineSourceBuildOptions, InstalledBuildOptions, ProjectPackageOptions,
    UbtMutexMode, UePlatform, engine_source_build_commands, installed_build_commands,
    project_package_commands,
};

fn argv(commands: &[ue_commands::UeCommand]) -> Vec<Vec<String>> {
    commands.iter().map(|command| command.argv()).collect()
}

#[test]
fn project_package_argv_matches_ueb_default_buildcookrun() {
    let options = ProjectPackageOptions {
        engine_root: PathBuf::from("C:/UE"),
        project: PathBuf::from("C:/ws/Project/AES.uproject"),
        archive_dir: PathBuf::from("C:/ws/Archive"),
        platform: UePlatform::Windows,
        configuration: Configuration::Development,
        mutex: UbtMutexMode::NoMutex,
        package_args: None,
        clean: false,
    };

    assert_eq!(
        argv(&project_package_commands(&options)),
        vec![
            vec![
                "C:/UE/Engine/Build/BatchFiles/RunUAT.bat",
                "BuildCookRun",
                "-project=C:/ws/Project/AES.uproject",
                "-archivedirectory=C:/ws/Archive",
                "-targetplatform=Win64",
                "-clientconfig=Development",
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
                "-ubtargs=-NoMutex",
            ]
            .into_iter()
            .map(String::from)
            .collect::<Vec<String>>()
        ]
    );
}

#[test]
fn project_package_derives_platform_configuration_mutex_and_clean() {
    let options = ProjectPackageOptions {
        engine_root: PathBuf::from("C:/UE"),
        project: PathBuf::from("C:/ws/Project/AES.uproject"),
        archive_dir: PathBuf::from("C:/ws/Archive"),
        platform: UePlatform::Linux,
        configuration: Configuration::Shipping,
        mutex: UbtMutexMode::Wait,
        package_args: None,
        clean: true,
    };
    let command = project_package_commands(&options)[0].argv();

    assert!(command.contains(&"-targetplatform=Linux".to_string()));
    assert!(command.contains(&"-clientconfig=Shipping".to_string()));
    assert!(command.contains(&"-ubtargs=-WaitMutex".to_string()));
    assert_eq!(command.last(), Some(&"-Clean".to_string()));
}

#[test]
fn project_package_preserves_explicit_package_args_ubtargs() {
    let options = ProjectPackageOptions {
        engine_root: PathBuf::from("C:/UE"),
        project: PathBuf::from("C:/ws/Project/AES.uproject"),
        archive_dir: PathBuf::from("C:/ws/Archive"),
        platform: UePlatform::Windows,
        configuration: Configuration::Shipping,
        mutex: UbtMutexMode::NoMutex,
        package_args: Some(vec![
            "-clientconfig=Test".into(),
            "-cook".into(),
            "-ubtargs=-Custom".into(),
        ]),
        clean: false,
    };
    let command = project_package_commands(&options)[0].argv();

    assert!(command.contains(&"-clientconfig=Test".to_string()));
    assert!(!command.contains(&"-clientconfig=Shipping".to_string()));
    assert!(command.contains(&"-ubtargs=-Custom".to_string()));
    assert!(!command.contains(&"-ubtargs=-NoMutex".to_string()));
}

#[test]
fn engine_source_build_argv_matches_ueb_three_steps() {
    let options = EngineSourceBuildOptions {
        engine_root: PathBuf::from("C:/UE"),
        platform: UePlatform::Windows,
        configuration: Configuration::Development,
        mutex: UbtMutexMode::NoMutex,
        gitdeps_threads: 256,
        gitdeps_cache: Some(PathBuf::from("C:/UE4_GITDEPS")),
        editor_target: "UnrealEditor".into(),
        extra_targets: vec!["UnrealPak".into()],
    };

    assert_eq!(
        argv(&engine_source_build_commands(&options)),
        vec![
            vec![
                "C:/UE/Engine/Binaries/DotNET/GitDependencies/win-x64/GitDependencies.exe",
                "--threads=256",
                "--force",
                "--cache=C:/UE4_GITDEPS",
            ],
            vec!["C:/UE/GenerateProjectFiles.bat"],
            vec![
                "C:/UE/Engine/Build/BatchFiles/Build.bat",
                "-Target=UnrealEditor Win64 Development -Quiet",
                "-Target=ShaderCompileWorker Win64 Development -Quiet",
                "-Target=UnrealPak Win64 Development -Quiet",
                "-NoMutex",
                "-FromMsBuild",
            ],
        ]
        .into_iter()
        .map(|row| row.into_iter().map(String::from).collect())
        .collect::<Vec<Vec<String>>>()
    );
}

#[test]
fn engine_source_build_can_wait_or_fail_fast_on_mutex() {
    let mut options = EngineSourceBuildOptions {
        engine_root: PathBuf::from("C:/UE"),
        platform: UePlatform::Windows,
        configuration: Configuration::Development,
        mutex: UbtMutexMode::Wait,
        gitdeps_threads: 8,
        gitdeps_cache: None,
        editor_target: "UnrealEditor".into(),
        extra_targets: Vec::new(),
    };
    assert!(
        engine_source_build_commands(&options)[2]
            .argv()
            .contains(&"-WaitMutex".to_string())
    );

    options.mutex = UbtMutexMode::FailFast;
    let command = engine_source_build_commands(&options)[2].argv();
    assert!(!command.contains(&"-NoMutex".to_string()));
    assert!(!command.contains(&"-WaitMutex".to_string()));
}

#[test]
fn installed_build_argv_matches_ueb_buildgraph_windows_and_linux_flags() {
    let windows = InstalledBuildOptions {
        engine_root: PathBuf::from("C:/UE"),
        output_dir: PathBuf::from("C:/ws/InstalledEngines"),
        platform: UePlatform::Windows,
    };
    assert_eq!(
        argv(&installed_build_commands(&windows)),
        vec![
            vec![
                "C:/UE/Engine/Build/BatchFiles/RunUAT.bat",
                "BuildGraph",
                "-target=Make Installed Build Win64",
                "-script=Engine/Build/InstalledEngineBuild.xml",
                "-set:WithMac=false",
                "-set:WithTVOS=false",
                "-set:WithLinux=false",
                "-set:WithLinuxArm64=false",
                "-set:WithAndroid=false",
                "-set:WithIOS=false",
                "-set:BuiltDirectory=C:/ws/InstalledEngines",
            ]
            .into_iter()
            .map(String::from)
            .collect::<Vec<String>>()
        ]
    );

    let with_linux = InstalledBuildOptions {
        platform: UePlatform::WindowsWithLinux,
        ..windows
    };
    let command = installed_build_commands(&with_linux)[0].argv();
    assert!(command.contains(&"-set:WithLinux=true".to_string()));
    assert!(command.contains(&"-set:WithLinuxArm64=true".to_string()));
}
