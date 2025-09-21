# Prompt gemini

## Purpose

We want this MCP server to be able to invoke `gemini-cli` from a pre-configured command which we will pass as either a CLI argument or an env var.

To begin with, we want to add a new MCP tool called `prompt-gemini`. This tool will receive `prompt` argument and will pass it to `gemini-cli` which should run in the headless mode. Once `gemini-cli` returns its response, we will pass the response back to the user as our response to the tool call.


## Workflow

1. Read `src/main.rs` and `src/tools.rs` to understand how our MCP server opertates by studying.
2. Read `spec/prompt_gemini.md` to understand how `gemini-cli` works in the headless mode.
3. Add a new command line argument for user to specify `gemnini-cli-command`. Make sure it supports reading from an env var too.
4. Add a new tool called `prompt-gemini` which runs the specified command and passes it the prompt from its argument. Make sure to include tests for it.
