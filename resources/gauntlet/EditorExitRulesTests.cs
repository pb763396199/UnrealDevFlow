using Gauntlet;

namespace Udf;

public sealed class EditorExitRulesTests : Gauntlet.SelfTest.BaseTestNode
{
    public override void TickTest()
    {
        CheckResult(
            StrictExitRules.Evaluate(new StrictExitObservation(true, false, 0, false, false, false)) == StrictExitOutcome.Passed,
            "natural exit with raw zero should pass");
        CheckResult(
            StrictExitRules.Evaluate(new StrictExitObservation(true, false, 0, false, true, false)) == StrictExitOutcome.Failed,
            "ensure failure should fail");
        CheckResult(
            StrictExitRules.Evaluate(new StrictExitObservation(true, false, 3, false, false, false)) == StrictExitOutcome.Failed,
            "non-zero raw exit should fail");
        CheckResult(
            StrictExitRules.Evaluate(new StrictExitObservation(true, true, 0, false, false, false)) == StrictExitOutcome.Failed,
            "killed process should fail even when normalized to zero");
        CheckResult(
            StrictExitRules.Evaluate(new StrictExitObservation(false, false, null, false, false, false)) == StrictExitOutcome.Unknown,
            "missing final process observation should be unknown");
        CheckResult(
            StrictExitRules.Evaluate(new StrictExitObservation(true, false, 0, true, false, false)) == StrictExitOutcome.Failed,
            "fatal error should fail");
        CheckResult(
            StrictExitRules.Evaluate(new StrictExitObservation(true, false, 0, false, false, true)) == StrictExitOutcome.Failed,
            "timeout should fail");
        CheckResult(
            StrictExitRules.Evaluate(new StrictExitObservation(true, false, 0, false, false, false)) != StrictExitOutcome.Failed,
            "ordinary errors are not represented as fatal by this rule");
        MarkComplete();
    }
}
