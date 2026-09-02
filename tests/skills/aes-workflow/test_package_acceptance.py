import json
import subprocess
import unittest
from pathlib import Path


REPO = Path(__file__).resolve().parents[3]


def cargo_test(test_name: str, test_target: str | None = None) -> None:
    command = ["cargo", "test"]
    if test_target:
        command.extend(["--test", test_target])
    command.extend([test_name, "--", "--exact"])
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


class PackageAcceptance(unittest.TestCase):
    def test_plugin_collection_expands_nested_uplugins_without_an_execution(self) -> None:
        cargo_test(
            "plugin_collection_expands_nested_uplugins_without_an_execution",
            "package_lifecycle",
        )

    def test_real_unreal_mcp_collection_is_ready_and_plans_78_no_mutex_steps(self) -> None:
        base = [
            "udf",
            "package",
        ]
        target = [
            "advanced",
            "plugin",
            "UnrealMCP",
            "--task",
            "neon-dev/unreal-mcp-functional-eval",
            "--format",
            "json",
        ]

        check = subprocess.run(
            [*base, "advanced", "plugin", "UnrealMCP", "--check", *target[3:]],
            cwd=REPO,
            text=True,
            encoding="utf-8",
            errors="replace",
            capture_output=True,
        )
        self.assertEqual(check.returncode, 0, check.stdout + check.stderr)
        check_document = json.loads(check.stdout)
        self.assertEqual(check_document["data"]["readiness"], "ready")
        self.assertNotIn("executionId", check_document["data"])

        plan = subprocess.run(
            [*base, "advanced", "plugin", "UnrealMCP", "--plan", *target[3:]],
            cwd=REPO,
            text=True,
            encoding="utf-8",
            errors="replace",
            capture_output=True,
        )
        self.assertEqual(plan.returncode, 0, plan.stdout + plan.stderr)
        plan_document = json.loads(plan.stdout)
        steps = plan_document["data"]["steps"]
        self.assertEqual(len(steps), 78)
        self.assertTrue(all("-NoMutex" in step["argv"] for step in steps))
        self.assertNotIn("executionId", plan_document["data"])

    def test_project_settings_parse(self) -> None:
        cargo_test("configure_inherits_project_packaging_baseline_and_detects_changes", "package_profile")

    def test_native_argv_mapping(self) -> None:
        cargo_test("project_profile_uses_native_project_settings_for_package_args", "package_commands")

    def test_cook_reuse_matrix(self) -> None:
        cargo_test("package_plan_exposes_the_saved_profile_without_a_second_show_command", "package_profile")

    def test_stale_profile(self) -> None:
        cargo_test("configure_inherits_project_packaging_baseline_and_detects_changes", "package_profile")

    def test_package_help_describes_cook_modes(self) -> None:
        cargo_test("package_help_describes_cook_modes", "cli_taxonomy")

    def test_cook_mode_is_fixed_per_profile(self) -> None:
        cargo_test("configure_can_fix_full_mode_and_requires_reason_for_existing_profile", "package_profile")

    def test_project_package_scope_is_explicit(self) -> None:
        cargo_test("legacy_plugin_entrypoint_only_returns_migration_guidance", "package_lifecycle")

    def test_package_disk_and_lineage_guard(self) -> None:
        cargo_test("clean_without_scope_is_an_inventory_only_operation", "package_lifecycle")
        cargo_test("delivery_reconciles_stale_udf_files_but_keeps_unowned_files")

    def test_project_package_argv_matches_ueb_default_buildcookrun(self) -> None:
        cargo_test(
            "project_package_argv_matches_ueb_default_buildcookrun",
            "package_commands",
        )

    def test_build_and_workspace_gain_planned_taxonomy_actions(self) -> None:
        cargo_test(
            "non_execution_queries_follow_the_shared_vocabulary",
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
