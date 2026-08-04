//! CLI command definitions for UnrealDevFlow
//!
//! 帮助文本一律写中文。这个工具的使用者是中文开发者和替他们干活的 AI 助手，
//! 命令名、参数名和取值保持英文（它们是要照着敲的），说明文字用中文。

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// clap 自带的段落标题和 -h/-V 说明是英文硬编码的。`next_help_heading` 换掉
/// 选项段的标题，`help_template` 换掉用法段的标题，`-h`/`-V` 则关掉自动版本
/// 换成自己声明的同名参数。`help` 子命令的说明在 main.rs 里用
/// `mut_subcommand` 补，derive 宏够不到它。
pub const HELP_TEMPLATE: &str = "{about}

用法：{usage}

{all-args}";

#[derive(Parser)]
#[command(name = "udf")]
#[command(about = "UE 插件并行开发工作流工具")]
#[command(version = env!("UDF_VERSION_LONG"))]
#[command(help_template = HELP_TEMPLATE)]
#[command(next_help_heading = "选项")]
#[command(subcommand_help_heading = "命令")]
#[command(disable_help_flag = true, disable_version_flag = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// 显示帮助
    #[arg(long, short = 'h', global = true, action = clap::ArgAction::Help)]
    pub help: Option<bool>,

    /// 显示版本号
    #[arg(long, short = 'V', action = clap::ArgAction::Version)]
    pub version: Option<bool>,

    /// 输出格式：human（默认）或 json。每个命令都认这个参数；
    /// json 模式下一条命令只往标准输出吐一个文档。
    #[arg(long, global = true, default_value = "human")]
    pub format: OutputFormat,

    /// 打开详细日志
    #[arg(long, short, global = true)]
    pub verbose: bool,
}

#[derive(Clone, Debug, clap::ValueEnum)]
pub enum OutputFormat {
    Json,
    Human,
}

#[derive(Clone, Debug, clap::ValueEnum, PartialEq)]
pub enum MergeStrategy {
    /// 把任务分支变基到 dev 上（推荐，历史是线性的）
    Rebase,
    /// 生成一个合并提交，保留任务分支的历史
    Merge,
    /// 把任务的所有提交压成一个
    Squash,
    /// 只允许快进，做不到就报错
    FfOnly,
}

#[derive(Subcommand)]
pub enum Commands {
    /// AgentWatcher 模块状态探测（只读）。
    #[command(name = "aw-status", hide = true)]
    AwStatus,

    /// 配置和检查你干活用的 UE 项目环境。
    Workspace {
        #[command(subcommand)]
        action: WorkspaceAction,
    },

    /// 创建、查看和收尾隔离的开发任务。
    Task {
        #[command(subcommand)]
        action: TaskAction,
    },

    /// 编译任务宿主或主项目，以及判断现在允不允许编。
    Build {
        #[command(subcommand)]
        action: BuildAction,
    },

    /// 给 AI 助手（opencode / copilot / codex / claude code）装、查、删
    /// UnrealDevFlow 的 skill。
    Skill {
        #[command(subcommand)]
        action: SkillAction,
    },
}

#[derive(Subcommand)]
pub enum BuildAction {
    /// 编译一个任务的宿主项目
    Task {
        /// 任务引用，形如 workspace/task-id。
        task_ref: String,

        /// 放到后台编，命令立刻返回
        #[arg(long)]
        background: bool,

        /// 互斥锁模式：auto（默认）/ wait / no-mutex
        ///
        /// auto    ：引擎的 Intermediate/Build/Shared 还没生成时用 -WaitMutex，
        ///           共享 PCH 就绪之后改用 -NoMutex。带 --validator 时偏向 -NoMutex。
        /// wait    ：始终 -WaitMutex（别人在编就排队等）
        /// no-mutex：始终 -NoMutex（并行，PCH 已缓存时最快）
        #[arg(long, value_enum, default_value = "auto")]
        mutex: crate::build_profile::MutexMode,

        /// 告诉工具这次编译是验证器或 CI 发起的，不是 IDE。
        #[arg(long)]
        validator: bool,

        /// 只编主插件的模块（给每个主插件传一个 `-Module=`）
        #[arg(long)]
        primary_only: bool,

        /// 编译严格度档位。默认 light。
        #[arg(long, value_enum, default_value = "light")]
        profile: crate::build_profile::BuildProfile,

        /// 改写 UBT 日志目录。默认 <host>/Logs/UBT/
        #[arg(long)]
        build_log_dir: Option<PathBuf>,
    },

    /// 编 workspace 的主 UE 项目，而不是任务宿主
    Project {
        /// workspace 名。配了不止一个 workspace 时必须给。
        #[arg(long)]
        workspace: Option<String>,

        /// 编译严格度档位。默认 light。
        #[arg(long, value_enum, default_value = "light")]
        profile: crate::build_profile::BuildProfile,

        /// 改写编辑器目标名。默认从 .uproject 推导。
        #[arg(long)]
        target: Option<String>,
    },

    /// 回答现在能不能启动一次受控编译（它自己永远不编）
    ///
    /// 结论是四个之一：ready / needsUserInput / blocked / deferred。
    Check {
        /// 任务引用，形如 workspace/task-id。不给就检查 workspace 的主项目，
        /// 而不是某个任务宿主。
        task_ref: Option<String>,

        /// workspace 名。不给任务引用时用它定位项目。
        #[arg(long)]
        workspace: Option<String>,

        /// 编译严格度档位。默认 light。
        #[arg(long, value_enum, default_value = "light")]
        profile: crate::build_profile::BuildProfile,

        /// 改写编辑器目标名。默认从 .uproject 推导。
        #[arg(long)]
        target: Option<String>,

        /// 顺便校验这条编译命令跟解析出来的项目对不对得上，在它真跑之前。
        #[arg(long)]
        build_command: Option<String>,
    },

    /// 检查一条命令是不是绕过了受控编译路径
    ///
    /// 判定为拒绝时退出码非零。
    Gate {
        /// 某个工具或 AI 助手打算执行的那条命令。
        command: String,
    },

    /// 查一个任务的编译状态
    Status {
        /// 任务引用，形如 workspace/task-id。不给就按最近动过的任务。
        task_ref: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum TaskAction {
    /// 建一个隔离的任务：每个主插件一个 git worktree，外加一个宿主项目。
    Create {
        /// 任务描述。没给 --prompt 时，它同时被存成用户原始需求。
        description: String,

        /// 自定义任务 ID（覆盖自动建议的那个）
        #[arg(long)]
        id: Option<String>,

        /// 每个主插件仓库里都用这个分支名。
        /// 默认是 task/<workspace>/<task-id>。
        #[arg(long)]
        branch: Option<String>,

        /// 每个主插件仓库都从这个 git 引用切分支。
        #[arg(long)]
        base_ref: Option<String>,

        /// 存进元数据的用户原始需求。不给就用任务描述顶上。
        #[arg(long)]
        prompt: Option<String>,

        /// workspace 名。配了不止一个 workspace 时必须给。
        #[arg(long)]
        workspace: Option<String>,

        /// 主插件名，多个用逗号隔开。不给就退回配置里那个
        /// 老式的 plugin_path 插件。
        #[arg(long, value_delimiter = ',')]
        primary: Option<Vec<String>>,

        /// 指定某个依赖从哪来，写成 `名字=engine` / `名字=project` /
        /// `名字=<绝对路径>`。可以给多次。
        #[arg(long = "override-dep", value_parser = parse_dep_override)]
        override_dep: Vec<DepOverride>,

        /// 不要问我，直接做
        #[arg(long, short = 'y')]
        yes: bool,
    },

    /// 列出所有任务
    List {
        /// 只看这个 workspace 下的任务
        #[arg(long)]
        workspace: Option<String>,
    },

    /// 告诉你这个任务接下来该做哪一步。
    Next {
        /// 任务引用，形如 workspace/task-id。不给且不产生歧义时按最近的任务。
        task_ref: Option<String>,
    },

    /// 把某个 UE 项目的 Junction 指到这个任务上（下次启动编辑器才生效）
    Switch {
        /// 任务引用，形如 workspace/task-id。写 "main" 表示切回主仓库。
        task_ref: String,

        /// 要改的 UE 项目路径，多个用逗号隔开。
        ///
        /// 只有 "main" 允许一次指向多个项目。任务绑死在它元数据里冻结的
        /// 那个项目上，点名别的项目会在动任何 Junction 之前就被拒绝。
        #[arg(long, value_delimiter = ',')]
        project: Option<Vec<PathBuf>>,

        /// 编辑器还开着也照切
        #[arg(long)]
        force: bool,

        /// 跳过自动重新生成 Visual Studio 工程文件这一步。
        #[arg(long)]
        skip_regen_project_files: bool,
    },

    /// 把一个任务合回它的主插件仓库
    ///
    /// ⚠️ 这条命令只合并，什么都不删。
    /// 合完之后你必须再跑一次 `task cleanup` 去清 worktree 和分支。
    ///
    /// 一个任务可以有多个主插件。用 --plugin 指定合哪一个，
    /// 或者用 --all 逆序逐个合，每个插件单独确认。
    Merge {
        /// 任务引用，形如 workspace/task-id。
        task_ref: String,

        /// 合并策略（必填，AI 助手必须先问用户）
        /// 可选：rebase、merge、squash、ff-only
        #[arg(long, value_enum)]
        strategy: MergeStrategy,

        /// 只合这一个主插件（任务有多个主插件且没给 --all 时必填）
        #[arg(long)]
        plugin: Option<String>,

        /// 按声明的逆序逐个合并每个主插件，每个单独确认
        #[arg(long)]
        all: bool,

        /// 有冲突也照合
        #[arg(long)]
        force: bool,

        /// 不要问我，直接做
        #[arg(long, short = 'y')]
        yes: bool,

        /// 只看会合什么，不真的合
        #[arg(long)]
        dry_run: bool,
    },

    /// 走一遍合并向导，把任务收掉。
    Finish {
        /// 任务引用，形如 workspace/task-id。不给且不产生歧义时按最近的任务。
        task_ref: Option<String>,

        /// 合并策略。不给就交互式问你。
        #[arg(long, value_enum)]
        strategy: Option<MergeStrategy>,

        /// 合并全部主插件。
        #[arg(long)]
        all: bool,

        /// 只合这一个主插件。
        #[arg(long)]
        plugin: Option<String>,

        /// 策略定下来之后就别再问了。
        #[arg(long, short = 'y')]
        yes: bool,
    },

    /// 合并验证过之后，清掉 worktree 和分支
    Cleanup {
        /// 任务引用，形如 workspace/task-id。
        task_ref: String,

        /// 不确认直接清
        #[arg(long)]
        force: bool,

        /// 不要问我，直接做
        #[arg(long, short = 'y')]
        yes: bool,
    },

    /// 不合并，直接删掉一个任务
    Delete {
        /// 任务引用，形如 workspace/task-id。
        task_ref: String,

        /// 不确认直接删
        #[arg(long)]
        force: bool,

        /// 不要问我，直接做
        #[arg(long, short = 'y')]
        yes: bool,

        /// 只看会删什么，不真的删
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
pub enum WorkspaceAction {
    /// 第一次用就跑这个：探测并登记一个 workspace，然后装上 AI skill。
    Init {
        /// workspace 名。不给就从项目路径里给你建议一个。
        #[arg(long)]
        workspace: Option<String>,

        /// 含有 .uproject 文件的 UE 项目目录。
        #[arg(long)]
        project: Option<PathBuf>,

        /// 项目插件根目录。不给就从项目的同级目录里推。
        #[arg(long)]
        plugins_root: Option<PathBuf>,

        /// 宿主根目录。不给默认是 <项目上级目录>/Hosts。
        #[arg(long)]
        hosts_root: Option<PathBuf>,

        /// UE 引擎路径。不给就按 .uproject 里的 EngineAssociation 推。
        #[arg(long)]
        engine_path: Option<PathBuf>,

        /// 不要问我，直接做。
        #[arg(long, short = 'y')]
        yes: bool,

        /// 初始化时不要装 AI skill。
        #[arg(long)]
        skip_skill_install: bool,
    },

    /// 新增或更新一个具名的 UE 项目 workspace。
    Add {
        /// 给这个 workspace 起的名字。
        name: String,
        /// 含有 .uproject 文件的 UE 项目目录。
        #[arg(long)]
        project: PathBuf,
        /// 宿主根目录。
        #[arg(long)]
        hosts_root: PathBuf,
        /// 项目插件根目录。
        #[arg(long)]
        plugins_root: PathBuf,
        /// UE 引擎路径。不给就从项目推。
        #[arg(long)]
        engine_path: Option<PathBuf>,
        /// 老式的默认主插件路径。
        #[arg(long)]
        plugin_path: Option<PathBuf>,
        /// 不要问我，直接做。
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// 列出已登记的 workspace。
    List,
    /// 检查一个 workspace 指的目录还在不在、对不对。
    Doctor {
        /// 要检查的 workspace 名。
        name: String,
        /// 把 plugins_root 底下每一个插件源都递归查一遍。
        #[arg(long)]
        deep: bool,
    },
    /// 注销一个 workspace。
    Remove {
        /// 要注销的 workspace 名。
        name: String,
        /// 不要问我，直接做。
        #[arg(long, short = 'y')]
        yes: bool,
    },

    /// 看每个 UE 项目现在指向哪个任务。
    Status,
}

#[derive(Subcommand)]
pub enum SkillAction {
    /// 把 skills/<名字>/SKILL.md 装到 .codex/ + .agents/ + .claude/ 和 opencode 的 skill 目录
    Install {
        /// 装到用户主目录（~/.codex/ + ~/.agents/ + ~/.claude/ + opencode），而不是当前项目
        #[arg(long, short = 'g')]
        global: bool,
        /// 项目根目录（默认当前目录）
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// 看哪些 AI 助手已经装了这个 skill（项目级和全局都看）
    List {
        /// 项目根目录（默认当前目录）
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// 删掉装好的 skill
    Remove {
        /// 删用户主目录里的那份，而不是当前项目的
        #[arg(long, short = 'g')]
        global: bool,
        /// 项目根目录（默认当前目录）
        #[arg(long)]
        project: Option<PathBuf>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum DepOverrideKind {
    Engine,
    Project,
    CustomPath(PathBuf),
}

#[derive(Clone, Debug, PartialEq)]
pub struct DepOverride {
    pub name: String,
    pub kind: DepOverrideKind,
}

fn parse_dep_override(raw: &str) -> std::result::Result<DepOverride, String> {
    let (name, value) = raw
        .split_once('=')
        .ok_or_else(|| format!("--override-dep '{}' 写法不对，应该是 名字=取值", raw))?;
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(format!("--override-dep '{}' 写法不对：名字是空的", raw));
    }
    let value = value.trim();
    let kind = match value {
        "engine" => DepOverrideKind::Engine,
        "project" => DepOverrideKind::Project,
        other => DepOverrideKind::CustomPath(PathBuf::from(other)),
    };
    Ok(DepOverride { name, kind })
}
