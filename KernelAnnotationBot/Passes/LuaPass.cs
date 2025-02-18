namespace KernelAnnotationBot.Passes;

public class LuaPass : AnnotationPassBase
{
    public override string Name => "Lua";

    protected override void AnalyzeInternal(IEnumerable<string> lines)
    {
        foreach (var (testcase, result) in FilterSingleLineResults(lines, "lua"))
        {
            int score = 0;

            if (result == "success") score = 1;

            AddTestcaseResult(testcase, score);
        }
    }
}
