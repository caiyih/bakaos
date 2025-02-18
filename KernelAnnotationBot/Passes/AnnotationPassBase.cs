using System.Collections.ObjectModel;
using System.Diagnostics;

namespace KernelAnnotationBot.Passes;

public abstract class AnnotationPassBase
{
    public abstract string Name { get; }

    private readonly Dictionary<string, TestcaseResult> _results = [];

    public ReadOnlyDictionary<string, TestcaseResult> TestResults => _results.AsReadOnly();

    public double TotalScore => TestResults.Values.Select(t => t.Score).Sum();

    public virtual void Analyze(string outputs)
    {
        AnalyzeInternal(outputs.Split('\n').Select(l => l.TrimEnd('\r')));
    }

    public static IEnumerable<(string, string[])> FilterMultiLineResults(IEnumerable<string> lines, string[]? matchHeaders = null)
    {
        string? caseName = null;
        List<string>? blockContent = null;

        foreach (var line in lines)
        {
            if (line.StartsWith("========== START "))
            {
                var startIndex = line.IndexOf("START ") + "START ".Length;
                var endIndex = line.LastIndexOf("==========");

                if (startIndex == -1 || endIndex == -1)
                    continue;

                var headerName = line[startIndex..endIndex];

                if (matchHeaders is not null && !matchHeaders.Any(headerName.StartsWith))
                    continue;

                if (blockContent is not null && caseName is not null)
                {
                    string name = caseName;
                    string[] contents = blockContent.ToArray();

                    caseName = null;
                    blockContent = null;

                    yield return (name, contents);
                }

                caseName = line[startIndex..endIndex].Trim();

                if (caseName is not null)
                {
                    blockContent = new List<string>();
                }
                else
                {
                    blockContent = null;
                }
            }
            else if (blockContent is not null)
            {
                Debug.Assert(caseName is not null);

                if (line.StartsWith("========== END "))
                {
                    string name = caseName;
                    string[] contents = blockContent.ToArray();

                    caseName = null;
                    blockContent = null;

                    yield return (name, contents);
                }
                else
                {
                    blockContent.Add(line);
                }
            }
        }
    }

    public static IEnumerable<(string, string)> FilterSingleLineResults(IEnumerable<string> lines, string testName, string header = "testcase") =>
        lines.Where(line => line.StartsWith(header))
            .Select(line => (line, line.Split(' ', StringSplitOptions.RemoveEmptyEntries)))
            .Where(u => u.Item2.Length >= 2 && u.Item2[1] == testName)
            .Select(u =>
            {
                string? result = u.Item2.Length >= 4 ? u.Item2.Last() : null;

                if (result is null) return ((string, string)?)null;
                int thirdSpace = u.line.IndexOf(u.Item2[2]);

                int resultPosition = u.line.LastIndexOf(result);

                if (thirdSpace == -1 || resultPosition == -1) return null;

                resultPosition--;

                Debug.Assert(u.line[resultPosition] is ' ', $"Expect space at {resultPosition}, line: \"{u.line}\"");

                while (u.line[resultPosition] == ' ')
                    resultPosition--;

                string caseName = u.line[thirdSpace..(resultPosition + 1)];

                return (caseName, result);
            })
            .Where(r => r.HasValue)
            .Select(r => r!.Value);

    protected abstract void AnalyzeInternal(IEnumerable<string> lines);

    protected void AddTestcaseResult(string testcase, double score, double? fullScore = null)
    {
        if (testcase is not null)
        {
            TestcaseResult result = new TestcaseResult
            {
                Name = testcase,
                Score = Math.Max(0, score),
                FullScore = fullScore,
            };

            if (!_results.TryAdd(testcase, result))
            {
                var oldResult = _results[testcase];

                if (result.Score > oldResult.Score)
                {
                    _results[testcase] = result;
                }
            }
        }
    }
}

public struct TestcaseResult
{
    public string Name { get; set; }
    public double Score { get; set; }
    public double? FullScore { get; set; }
}
