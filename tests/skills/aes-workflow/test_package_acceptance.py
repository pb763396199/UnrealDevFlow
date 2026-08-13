import subprocess
import unittest
from pathlib import Path


REPO = Path(__file__).resolve().parents[3]


def cargo_test(test_name: str, test_target: str | None = None) -> None:
    command = ["cargo", "test"]
    if test_target:
        command.extend(["--test", test_target])
    command.extend([test_name, "--", "--exact"])
    completed = subprocess.run(command, cwd=REPO, text=True, capture_output=True)
    if completed.returncode != 0:
        raise AssertionError(completed.stdout + completed.stderr)


class PackageAcceptance(unittest.TestCase):
    def test_project_package_argv_matches_ueb_default_buildcookrun(self) -> None:
        cargo_test(
            "project_package_argv_matches_ueb_default_buildcookrun",
            "package_commands",
        )

    def test_build_and_workspace_gain_planned_taxonomy_actions(self) -> None:
        cargo_test(
            "build_and_workspace_gain_planned_taxonomy_actions",
            "cli_taxonomy",
        )

    def test_engine_source_build_argv_matches_ueb_three_steps(self) -> None:
        cargo_test(
            "engine_source_build_argv_matches_ueb_three_steps",
            "package_commands",
        )

    def test_package_clean_refuses_a_recorded_path_outside_managed_artifacts(self) -> None:
        cargo_test(
            "package_clean_refuses_a_recorded_path_outside_managed_artifacts",
            "package_lifecycle",
        )

    def test_same_named_secondary_commands_share_the_same_help_text(self) -> None:
        cargo_test(
            "same_named_secondary_commands_share_the_same_help_text",
            "cli_taxonomy",
        )

    def test_isolated_plugin_matrix_never_takes_engine_global_mutex(self) -> None:
        cargo_test(
            "commands::package::tests::isolated_plugin_matrix_never_takes_engine_global_mutex"
        )


if __name__ == "__main__":
    unittest.main()
