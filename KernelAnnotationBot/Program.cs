using System.Collections.Immutable;
using KernelAnnotationBot.Passes;

namespace KernelAnnotationBot;

static class Program
{
    static readonly List<AnnotationPassBase> annotationPasses =
    [
        new LibCTestPass(),
        new LuaPass(),
        new BusyBoxPass(),
    ];

    static void Main(string[] args)
    {
        string? filePath = null;
        string? basicResultPath = null;

        foreach (var arg in args)
        {
            string[] splited = arg.Split('=');
            string key = splited.First();

            var valArray = arg.SkipWhile(c => c is not '=').SkipWhile(c => c is '=').ToArray();
            string? val = valArray.Length > 0 ? new string(valArray) : null;

            switch (key)
            {
                case "-f":
                case "--file":
                    filePath = val;
                    break;

                case "-b":
                case "--basic":
                    basicResultPath = val;
                    break;

                default:
                    Console.WriteLine($"Unrecognized key: \"{key}\"");
                    break;
            }
        }

        if (filePath is not null)
        {
            string fileContent = File.ReadAllText(filePath);
            string? basicResult = basicResultPath is null || !File.Exists(basicResultPath) ? null : File.ReadAllText(basicResultPath);

            Analyze(fileContent, basicResult);
        }
        else
        {
            Console.WriteLine("File path not specified");
        }

        DisplayAnnotationResult();
    }

    static void Analyze(string content, string? basicResult = null)
    {
        foreach (var pass in annotationPasses)
        {
            pass.Analyze(content);
        }

        if (basicResult is not null)
        {
            var basicPass = new BasicPass();
            basicPass.Analyze(basicResult);
            annotationPasses.Add(basicPass);
        }
    }

    private static void DisplayAnnotationResult()
    {
        double totalScore = annotationPasses.Select(p => p.TotalScore).Sum();

        Console.WriteLine($"Total score of all tests: {totalScore:F2}");

        int padding = annotationPasses.Select(p => p.Name.Length).Max();
        foreach (var pass in annotationPasses)
        {
            Console.WriteLine($"Score of {pass.Name.PadRight(padding)}: {pass.TotalScore:F2}");
        }

        Console.WriteLine();

        foreach (var pass in annotationPasses)
        {
            Console.WriteLine($"Detailed result for {pass.Name}:");
            Console.WriteLine($"    Score       Testcase");

            foreach (var testcase in pass.TestResults)
            {
                double score = testcase.Value.Score;
                string scoreString = score.ToString("F2");

                if (testcase.Value.FullScore is double fullScore && score >= fullScore)
                {
                    scoreString = $"{scoreString}(full)";
                }

                Console.WriteLine($"    {scoreString,-10}  {testcase.Key}");
            }

            Console.WriteLine();
        }
    }
}
