import subprocess
import unittest
from pathlib import Path


REPO = Path(__file__).resolve().parents[3]


def run(command: list[str]) -> None:
    completed = subprocess.run(
        command,
        cwd=REPO,
        text=True,
        encoding="utf-8",
        errors="replace",
        capture_output=True,
    )
    if completed.returncode != 0:
        raise AssertionError(completed.stdout + completed.stderr)


def cargo_test(test_name: str, test_target: str, *, exact: bool = True) -> None:
    command = ["cargo", "test", "--test", test_target, test_name]
    if exact:
        command.extend(["--", "--exact"])
    run(command)


class CreateAcceptance(unittest.TestCase):
    def test_task_create_allows_untracked_main_checkout_changes(self) -> None:
        cargo_test(
            "task_create_allows_untracked_main_checkout_changes",
            "create_sources",
        )

    def test_task_create_allows_unstaged_tracked_main_checkout_changes(self) -> None:
        cargo_test(
            "task_create_allows_unstaged_tracked_main_checkout_changes",
            "create_sources",
        )

    def test_task_create_allows_staged_tracked_main_checkout_changes(self) -> None:
        cargo_test(
            "task_create_allows_staged_tracked_main_checkout_changes",
            "create_sources",
        )

    def test_task_create_rejects_unmerged_conflicts(self) -> None:
        cargo_test("task_create_rejects_unmerged_conflicts", "create_sources")

    def test_task_create_dirty_checkout_docs_are_consistent(self) -> None:
        cargo_test("task_create_dirty_checkout_docs_are_consistent", "create_sources")

    def test_create_sources_regression_suite(self) -> None:
        cargo_test("create_", "create_sources", exact=False)

    def test_rust_quality_gates_pass(self) -> None:
        run(["cargo", "fmt", "--all", "--", "--check"])
        run([
            "cargo",
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--locked",
            "--",
            "-D",
            "warnings",
        ])
        run(["cargo", "test"])


if __name__ == "__main__":
    unittest.main()
