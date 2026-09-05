namespace Udf;

public enum StrictExitOutcome
{
    Passed,
    Failed,
    Unknown,
}

public readonly record struct StrictExitObservation(
    bool HasExited,
    bool WasKilled,
    int? RawExitCode,
    bool HasFatal,
    bool HasEnsureFailure,
    bool TimedOut);

public static class StrictExitRules
{
    public static StrictExitOutcome Evaluate(StrictExitObservation observation)
    {
        if (observation.TimedOut || observation.WasKilled || observation.HasFatal || observation.HasEnsureFailure)
        {
            return StrictExitOutcome.Failed;
        }

        if (!observation.HasExited || observation.RawExitCode is null)
        {
            return StrictExitOutcome.Unknown;
        }

        return observation.RawExitCode.Value == 0
            ? StrictExitOutcome.Passed
            : StrictExitOutcome.Failed;
    }
}
