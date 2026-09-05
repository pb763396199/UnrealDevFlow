using System.Collections.Generic;
using System.IO;
using Gauntlet;
using UnrealBuildTool;

namespace Udf;

public sealed class EditorExit : UnrealTestNode<UnrealTestConfiguration>
{
    private readonly UnrealTestContext testContext;

    public EditorExit(UnrealTestContext context)
        : base(context)
    {
        testContext = context;
    }

    public override UnrealTestConfiguration GetConfiguration()
    {
        UnrealTestConfiguration config = base.GetConfiguration();
        config.ClearRoles();
        UnrealTestRole role = config.RequireRole(
            config.CookedEditor ? UnrealTargetRole.CookedEditor : UnrealTargetRole.Editor);
        // UDF passes the synchronization commands through a test-only Gauntlet
        // parameter.  Reading it here lets the node place the commands before
        // its own final quit marker; regular -ExecCmds is appended by UAT after
        // role configuration and would otherwise run after QUIT_EDITOR.
        string syncCommands = testContext.TestParams?.ParseValue("UdfSyncCmds", "") ?? "";
        if (!string.IsNullOrWhiteSpace(syncCommands))
        {
            role.CommandLineParams.Add("execcmds", syncCommands);
        }
        role.CommandLineParams.Add("execcmds", "QUIT_EDITOR");
        return config;
    }

    protected override UnrealProcessResult GetExitCodeAndReason(
        StopReason reason,
        UnrealLog log,
        UnrealRoleArtifacts artifacts,
        out string exitReason,
        out int exitCode)
    {
        UnrealProcessResult nativeResult = base.GetExitCodeAndReason(
            reason, log, artifacts, out exitReason, out exitCode);
        IAppInstance app = artifacts.AppInstance;
        bool hasFatal = nativeResult == UnrealProcessResult.EncounteredFatalError
            || log.FatalError != null;
        bool hasEnsureFailure = nativeResult == UnrealProcessResult.EncounteredEnsure;
        StrictExitObservation observation = new(
            app.HasExited,
            app.WasKilled,
            app.HasExited ? app.ExitCode : null,
            hasFatal,
            hasEnsureFailure,
            reason == StopReason.MaxDuration);
        StrictExitOutcome strict = StrictExitRules.Evaluate(observation);

        if (strict == StrictExitOutcome.Passed)
        {
            exitReason = $"Strict natural exit (raw UE ExitCode={app.ExitCode})";
            exitCode = app.ExitCode;
            return UnrealProcessResult.ExitOk;
        }

        if (strict == StrictExitOutcome.Unknown)
        {
            exitReason = "Strict natural exit could not obtain a final process observation";
            exitCode = -1;
            return UnrealProcessResult.Unknown;
        }

        exitReason = $"Strict natural exit failed (raw UE ExitCode={app.ExitCode}, WasKilled={app.WasKilled}, native={nativeResult})";
        exitCode = app.HasExited ? app.ExitCode : -1;
        return nativeResult == UnrealProcessResult.EncounteredFatalError
            ? nativeResult
            : UnrealProcessResult.TestFailure;
    }

    public override ITestReport CreateReport(
        TestResult result,
        UnrealTestContext context,
        UnrealBuildSource build,
        IEnumerable<UnrealRoleResult> roleResults,
        string artifactPath)
    {
        Directory.CreateDirectory(artifactPath);
        string reportPath = Path.Combine(artifactPath, "udf-editor-exit.json");
        var roles = new List<object>();
        foreach (UnrealRoleResult role in roleResults)
        {
            roles.Add(new
            {
                role = role.Artifacts.SessionRole.RoleType.ToString(),
                nativeResult = role.ProcessResult.ToString(),
                rawExitCode = role.Artifacts.AppInstance.HasExited
                    ? role.Artifacts.AppInstance.ExitCode
                    : (int?)null,
                hasExited = role.Artifacts.AppInstance.HasExited,
                wasKilled = role.Artifacts.AppInstance.WasKilled,
                log = role.Artifacts.LogPath,
            });
        }

        File.WriteAllText(
            reportPath,
            System.Text.Json.JsonSerializer.Serialize(new
            {
                result = result.ToString(),
                roles,
            }, new System.Text.Json.JsonSerializerOptions { WriteIndented = true }));
        return base.CreateReport(result, context, build, roleResults, artifactPath);
    }
}
