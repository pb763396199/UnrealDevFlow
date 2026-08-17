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
    def test_create_allows_untracked_files_in_primary_source_without_copying_them(self) -> None:
        cargo_test(
            "create_allows_untracked_files_in_primary_source_without_copying_them",
            "create_sources",
        )

    def test_create_still_rejects_tracked_changes_when_untracked_files_are_present(self) -> None:
        cargo_test(
            "create_still_rejects_tracked_changes_when_untracked_files_are_present",
            "create_sources",
        )

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
