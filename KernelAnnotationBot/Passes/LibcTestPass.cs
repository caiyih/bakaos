namespace KernelAnnotationBot.Passes;

public class LibCTestPass : AnnotationPassBase
{
    public override string Name => "Libc-test";

    protected override void AnalyzeInternal(IEnumerable<string> lines)
    {
        foreach (var (testcase, contents) in FilterMultiLineResults(lines, ["entry-dynamic.exe", "entry-static.exe"]))
        {
            int score = 0;

            if (contents.Any(line => line == "Pass!"))
            {
                score = 1;
            }

            AddTestcaseResult(testcase, score, 1);
        }
    }
}
