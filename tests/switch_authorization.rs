use std::fs;
use std::path::PathBuf;

fn repo_file(relative_path: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative_path);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

#[test]
fn switch_intent_authorizes_execution() {
    let expected_rules = [
        (
            "AGENTS.md",
            "用户表达对当前任务执行 `switch` 的意图，就视为已经授权",
        ),
        (
            "CLAUDE.md",
            "用户表达对当前任务执行 `switch` 的意图，就视为已经授权",
        ),
        (
            ".github/copilot-instructions.md",
            "When the user expresses intent to switch the current task, that message is the authorization",
        ),
        (
            "skill/SKILL.md",
            "用户表达对当前任务执行 `switch` 的意图，就视为已经授权",
        ),
        (
            "skills/unrealdevflow/SKILL.md",
            "用户表达对当前任务执行 `switch` 的意图，就视为已经授权",
        ),
    ];

    for (relative_path, expected_rule) in expected_rules {
        let content = repo_file(relative_path);
        assert!(
            content.contains(expected_rule),
            "{relative_path} must treat the user's switch intent as authorization"
        );
    }

    let source = repo_file("src/commands/switch.rs");
    assert!(
        !source.contains("dialoguer::Confirm"),
        "task switch must not open a second confirmation dialog"
    );
    assert!(
        !source.contains("Continue with switch?"),
        "task switch must not ask for confirmation after explicit invocation"
    );
    assert!(
        source.contains("Junction switch will only take effect on next Editor launch."),
        "the running-editor warning must remain"
    );
}

#[test]
fn other_destructive_confirmations_unchanged() {
    let agents = repo_file("AGENTS.md");
    assert!(agents.contains("merge 命令的 `--strategy` 是必填参数，不询问用户就报错"));
    assert!(agents.contains("等用户明确确认后，再 cleanup"));

    let skill = repo_file("skills/unrealdevflow/SKILL.md");
    assert!(skill.contains("`--strategy` 是**必填**的，而且你**必须问用户**"));
    assert!(skill.contains("用户确认合并结果没问题之后，才做这一步"));
}

#[test]
fn no_conflicting_switch_guidance() {
    let forbidden_rules = [
        "不要自己执行 switch",
        "不要自动 switch",
        "agent 不自己执行 switch",
        "user runs, not agent",
        "就算用户说了「帮我切」",
        "必须用户先关掉 UE 编辑器，再授权切换",
        "Tell user to run `udf task switch",
    ];

    for relative_path in [
        "AGENTS.md",
        "CLAUDE.md",
        ".github/copilot-instructions.md",
        "skill/SKILL.md",
        "skills/unrealdevflow/SKILL.md",
    ] {
        let content = repo_file(relative_path);
        for forbidden_rule in forbidden_rules {
            assert!(
                !content.contains(forbidden_rule),
                "{relative_path} still contains conflicting guidance: {forbidden_rule}"
            );
        }
    }
}
