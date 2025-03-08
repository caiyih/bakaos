using GitHub;
using GitHub.Octokit.Client;
using GitHub.Octokit.Client.Authentication;
using Microsoft.Kiota.Abstractions;

namespace KernelAnnotationBot;

public static class CommentHandler
{
    private static string GetToken()
    {
        const string TokenEnvName = "GITHUB_TOKEN";

        var envToken = Environment.GetEnvironmentVariable(TokenEnvName);

        if (envToken is string token && token.Length > 0)
        {
            return token;
        }

        throw new InvalidOperationException("Must provide token.");
    }

    public static (string, string, string) GetActionsContext()
    {
        const string RepositoryEnvName = "GITHUB_REPOSITORY";
        const string ShaEnvName = "GITHUB_SHA";

        string? envRepository = Environment.GetEnvironmentVariable(RepositoryEnvName);
        string? envSha = Environment.GetEnvironmentVariable(ShaEnvName);

        if (envRepository is string repository && envSha is string sha)
        {
            string[] splitedRepository = repository.Split('/');

            string owner = splitedRepository.First();
            string repo = splitedRepository.Last();

            return (owner, repo, sha);
        }

        throw new InvalidOperationException("One or more environment variable not provided: GITHUB_REPOSITORY and GITHUB_SHA");
    }

    public static void Create(string content)
    {
        string token = GetToken();

        TokenAuthProvider authProvider = new TokenAuthProvider(new TokenProvider(token));
        IRequestAdapter requestAdapter = RequestAdapter.Create(authProvider);
        GitHubClient gitHubClient = new GitHubClient(requestAdapter);

        var (owner, repo, sha) = GetActionsContext();

        var requestBody = new GitHub.Repos.Item.Item.Commits.Item.Comments.CommentsPostRequestBody
        {
            Body = content,
        };

        gitHubClient.Repos[owner][repo].Commits[sha].Comments.PostAsync(requestBody)
            .Wait();
    }
}