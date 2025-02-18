using System.Text.Json;
using System.Text.Json.Serialization;
using System.Text.Json.Serialization.Metadata;

namespace KernelAnnotationBot.Passes;

public class BasicPass : AnnotationPassBase
{
    public override string Name => "Basic";

    public override void Analyze(string outputs)
    {
        if (outputs is null)
            return;

        try
        {
            var results = JsonSerializer.Deserialize(outputs, BasicTestResultSerializationContext.Default.BasicTestcaseArray);

            if (results is not null)
            {
                foreach (var result in results)
                {
                    AddTestcaseResult(result.Name, result.Score, result.FullScore);
                }
            }
        }
        catch (Exception)
        {
            // ignore
        }
    }

    protected override void AnalyzeInternal(IEnumerable<string> lines)
    {
    }
}

struct BasicTestcase
{
    [JsonPropertyName("name")]
    public string Name { get; set; } = null!;

    [JsonPropertyName("passed")]
    public double Score { get; set; }

    [JsonPropertyName("all")]
    public double? FullScore { get; set; } = null;

    public BasicTestcase()
    {
    }
}

[JsonSerializable(typeof(BasicTestcase[]))]
partial class BasicTestResultSerializationContext : JsonSerializerContext
{
}
