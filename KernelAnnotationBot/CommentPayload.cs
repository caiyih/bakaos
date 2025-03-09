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

    public string? LogLevel { get; set; }

    public ImmutableList<AnnotationPassBase> TestPasses { get; set; } = null!;

    public double? FailThreshold { get; set; }

    public override string ToString()
    {
        if (TestPasses is null)
        {
            throw new InvalidOperationException("Test passes is null");
        }

        StringBuilder builder = new StringBuilder();

        builder.AppendLine(Title ?? "# Test result annotation");
        builder.AppendLine();

        double totalScore = TestPasses.Select(p => p.TotalScore).Sum();

        builder.AppendLine($"- Target: {Target}");
        builder.AppendLine();
        builder.AppendLine($"- Profile: {Profile}");
        builder.AppendLine();
        builder.AppendLine($"- Total Score: {totalScore:F2}");
        builder.AppendLine();

        if (FailThreshold is not null)
        {
            string message;

            if (totalScore > FailThreshold)
            {
                message = $"✅ Test coverage improved! Great job! 👍 (Increased by {totalScore - FailThreshold} points)";
            }
            else if (totalScore == FailThreshold)
            {
                message = $"✔️ Test coverage meets the requirement. Keep up the good work! 😊";
            }
            else
            {
                message = $"❗ Test coverage did not meet the target. (Short by {FailThreshold - totalScore} points)\nDon't give up, try to find out where the bug is! 🚀";
            }

            builder.AppendLine(message);
            builder.AppendLine();
        }

        builder.AppendLine("<details>");
        builder.AppendLine("<summary>Click for details</summary>");
        builder.AppendLine();
        {
            if (LogLevel is not null)
            {
                builder.AppendLine($"- Logging: {LogLevel}");
                builder.AppendLine();
            }

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
