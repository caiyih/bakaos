namespace KernelAnnotationBot.Passes;

public class BusyBoxPass : AnnotationPassBase
{
    public override string Name => "Busybox";

    protected override void AnalyzeInternal(IEnumerable<string> lines)
    {
        foreach (var (testcase, result) in FilterSingleLineResults(lines, "busybox"))
        {
            int score = 0;

            if (result == "success") score = 1;

            AddTestcaseResult(testcase, score);
        }
    }
}
