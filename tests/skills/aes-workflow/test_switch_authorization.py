from pathlib import Path
import unittest


REPO = Path(__file__).resolve().parents[3]


def repo_file(relative_path: str) -> str:
    return (REPO / relative_path).read_text(encoding="utf-8")


class SwitchAuthorizationTests(unittest.TestCase):
    def test_switch_intent_authorizes_execution(self) -> None:
        expected_rules = {
            "AGENTS.md": "用户表达对当前任务执行 `switch` 的意图，就视为已经授权",
            "CLAUDE.md": "用户表达对当前任务执行 `switch` 的意图，就视为已经授权",
            ".github/copilot-instructions.md": (
                "When the user expresses intent to switch the current task, "
                "that message is the authorization"
            ),
            "skill/SKILL.md": "用户表达对当前任务执行 `switch` 的意图，就视为已经授权",
            "skills/unrealdevflow/SKILL.md": (
                "用户表达对当前任务执行 `switch` 的意图，就视为已经授权"
            ),
        }

        for relative_path, expected_rule in expected_rules.items():
            with self.subTest(relative_path=relative_path):
                self.assertIn(expected_rule, repo_file(relative_path))

        source = repo_file("src/commands/switch.rs")
        self.assertNotIn("dialoguer::Confirm", source)
        self.assertNotIn("Continue with switch?", source)
        self.assertIn(
            "Junction switch will only take effect on next Editor launch.", source
        )

    def test_other_destructive_confirmations_unchanged(self) -> None:
        agents = repo_file("AGENTS.md")
        self.assertIn(
            "merge 命令的 `--strategy` 是必填参数，不询问用户就报错", agents
        )
        self.assertIn("等用户明确确认后，再 cleanup", agents)

        skill = repo_file("skills/unrealdevflow/SKILL.md")
        self.assertIn("`--strategy` 是**必填**的，而且你**必须问用户**", skill)
        self.assertIn("用户确认合并结果没问题之后，才做这一步", skill)

    def test_no_conflicting_switch_guidance(self) -> None:
        forbidden_rules = (
            "不要自己执行 switch",
            "不要自动 switch",
            "agent 不自己执行 switch",
            "user runs, not agent",
            "就算用户说了「帮我切」",
            "必须用户先关掉 UE 编辑器，再授权切换",
            "Tell user to run `udf task switch",
        )

        for relative_path in (
            "AGENTS.md",
            "CLAUDE.md",
            ".github/copilot-instructions.md",
            "skill/SKILL.md",
            "skills/unrealdevflow/SKILL.md",
        ):
            content = repo_file(relative_path)
            for forbidden_rule in forbidden_rules:
                with self.subTest(
                    relative_path=relative_path, forbidden_rule=forbidden_rule
                ):
                    self.assertNotIn(forbidden_rule, content)


if __name__ == "__main__":
    unittest.main()
