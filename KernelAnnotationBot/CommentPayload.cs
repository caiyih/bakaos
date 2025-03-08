using System.Collections.Immutable;
using System.Text;

namespace KernelAnnotationBot.Passes;

public class CommentPayload
{
    public string? Title { get; set; }

    public string Target { get; set; } = null!;

    public string profile = null!;

    public string Profile
    {
        get => profile;
        set
        {
            if (value is null)
            {
                return;
            }

            if (value.Contains("release") && value != "release")
            {
                profile = $"release({value})";
            }
            else
            {
                profile = value;
            }
        }
    }

    public string LogLevel { get; set; } = null!;

    public ImmutableList<AnnotationPassBase> TestPasses { get; set; } = null!;

    public override string ToString()
    {
        if (TestPasses is null)
        {
            throw new InvalidOperationException("Test passes is null");
        }

        StringBuilder builder = new StringBuilder();

        builder.AppendLine(Title ?? "# Test result annotation");
        builder.AppendLine();

        builder.AppendLine($"- Target: {Target}");
        builder.AppendLine();
        builder.AppendLine($"- Profile: {Profile}");
        builder.AppendLine();
        builder.AppendLine($"- Total Score: {TestPasses.Select(p => p.TotalScore).Sum():F2}");
        builder.AppendLine();

        builder.AppendLine("<details>");
        builder.AppendLine("<summary>Click for details</summary>");
        builder.AppendLine();
        {
            builder.AppendLine($"- Logging: {LogLevel}");
            builder.AppendLine();

            {
                int padding = TestPasses.Select(p => p.Name.Length).Max();
                foreach (var pass in TestPasses)
                {
                    builder.AppendLine($"- {pass.Name.PadRight(padding)}: {pass.TotalScore:F2}");
                    builder.AppendLine();
                }
            }

            {
                foreach (var pass in TestPasses)
                {
                    builder.AppendLine($"## Detailed result for {pass.Name}:");

                    if (pass.TestResults.Count == 0)
                    {
                        builder.AppendLine("*Skipped for no content*");
                    }
                    else
                    {
                        builder.AppendLine($"| Score | Testcase |");
                        builder.AppendLine($"|------:|----------|");

                        foreach (var testcase in pass.TestResults)
                        {
                            double score = testcase.Value.Score;
                            string scoreString = score.ToString("F2");

                            if (testcase.Value.FullScore is double fullScore && score >= fullScore)
                            {
                                scoreString = $"{scoreString}(full)";
                            }

                            builder.AppendLine($"|{scoreString}|{testcase.Key}|");
                        }
                    }

                    builder.AppendLine();
                }
            }
        }
        builder.AppendLine("</details>");


        return builder.ToString();
    }
}
