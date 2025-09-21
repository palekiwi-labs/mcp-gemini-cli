# Prompt Gemini MCP Tool - Analysis Report & Execution Plan

## Project Overview

This document contains the analysis and implementation plan for adding a `prompt-gemini` MCP tool to the existing MCP server. The goal is to enable the MCP server to invoke `gemini-cli` in headless mode and return responses to MCP clients.

## Current State Analysis

### Codebase Structure
- **Framework**: Uses `rmcp` crate (v0.6.4) for MCP protocol implementation
- **Server Type**: SSE (Server-Sent Events) server running on localhost:8000
- **Architecture**: Split between `src/main.rs` (server setup) and `src/tools.rs` (tool implementations)
- **Tool Pattern**: Tools defined using `#[tool]` macro with structured arguments
- **Current Tools**: FileSystem operations (list_files, read_file, write_file, get_file_info)

### Dependencies
Current dependencies include:
- `rmcp` with server, macros, transport-sse-server, schemars features
- `tokio` with async runtime and filesystem support
- `serde`/`serde_json` for serialization
- `anyhow` for error handling
- `tracing` for logging
- `axum` for HTTP server
- `schemars` for JSON schema generation

### MCP Tool Implementation Pattern
```rust
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ToolArgs {
    pub field: String,
}

#[tool(description = "Tool description")]
async fn tool_name(
    &self,
    Parameters(args): Parameters<ToolArgs>,
) -> Result<CallToolResult, McpError> {
    // Implementation
    Ok(CallToolResult::success(vec![Content::text(response)]))
}
```

## Gemini CLI Integration Analysis

### Headless Mode Capabilities
From the documentation analysis:
- **Primary Command**: `gemini --prompt "your prompt here"`
- **Alternative Input**: `echo "prompt" | gemini` (stdin)
- **Output Formats**: 
  - Text (default): Direct response string
  - JSON: Structured response with stats and metadata
- **Exit Codes**: Standard Unix exit codes for error handling

### Command Structure
```bash
# Basic usage
gemini --prompt "What is machine learning?"

# With JSON output
gemini --prompt "Explain Docker" --output-format json

# With additional options
gemini --prompt "Review code" --model gemini-2.5-flash --yolo
```

### JSON Response Schema
```json
{
  "response": "string",
  "stats": {
    "models": { /* per-model usage */ },
    "tools": { /* tool execution stats */ },
    "files": { /* file modification stats */ }
  },
  "error": { /* present only on error */ }
}
```

## Implementation Plan

### Phase 1: Dependencies & CLI Setup

#### 1.1 Update Cargo.toml
Add required dependencies:
```toml
clap = { version = "4.0", features = ["derive", "env"] }
```

#### 1.2 Command Line Arguments Structure
```rust
#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    /// Path or command to gemini-cli executable
    #[arg(long, env = "GEMINI_CLI_COMMAND", default_value = "gemini")]
    gemini_cli_command: String,
}
```

### Phase 2: Tool Implementation

#### 2.1 Argument Structure
```rust
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PromptGeminiArgs {
    /// The prompt to send to Gemini CLI
    pub prompt: String,
}
```

#### 2.2 Enhanced FileSystem Structure
```rust
#[derive(Clone)]
pub struct FileSystem {
    tool_router: ToolRouter<FileSystem>,
    gemini_cli_command: String,
}
```

#### 2.3 Core Tool Implementation
```rust
#[tool(description = "Send a prompt to Gemini CLI and return the response")]
async fn prompt_gemini(
    &self,
    Parameters(args): Parameters<PromptGeminiArgs>,
) -> Result<CallToolResult, McpError> {
    // Execute gemini-cli command
    // Handle output and errors
    // Return formatted response
}
```

### Phase 3: Command Execution Strategy

#### 3.1 Subprocess Execution
```rust
use tokio::process::Command;

let output = Command::new(&self.gemini_cli_command)
    .arg("--prompt")
    .arg(&args.prompt)
    .output()
    .await?;
```

#### 3.2 Error Handling Matrix
| Scenario | Handling |
|----------|----------|
| Command not found | Return clear "gemini-cli not found" error |
| Non-zero exit code | Include stderr in error response |
| Timeout | Implement reasonable timeout (30s default) |
| Empty output | Handle gracefully with appropriate message |

#### 3.3 Output Processing
- Check exit status first
- Parse stdout as response content
- Include stderr in error cases
- Consider JSON vs text output format handling

### Phase 4: Server Integration

#### 4.1 Main.rs Modifications
```rust
use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    
    // Pass gemini_cli_command to FileSystem::new()
    let ct = sse_server.with_service(|| FileSystem::new(args.gemini_cli_command.clone()));
}
```

#### 4.2 Backward Compatibility
- Maintain existing FileSystem constructor for tests
- Ensure existing tools continue to work unchanged
- Add new constructor variant that accepts gemini command

### Phase 5: Testing Strategy

#### 5.1 Unit Tests
```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_prompt_gemini_success() {
        // Mock successful gemini-cli execution
    }
    
    #[tokio::test]
    async fn test_prompt_gemini_command_not_found() {
        // Test error handling for missing command
    }
    
    #[tokio::test]
    async fn test_prompt_gemini_execution_error() {
        // Test error handling for failed execution
    }
}
```

#### 5.2 Integration Tests
- Test with dummy gemini-cli script
- Verify MCP protocol compliance
- Test argument parsing and environment variables
- Error scenario testing

#### 5.3 Manual Testing Checklist
- [ ] Basic prompt execution
- [ ] Error handling (command not found)
- [ ] Environment variable configuration
- [ ] CLI argument configuration
- [ ] MCP client integration
- [ ] Performance with long prompts

### Phase 6: Security & Performance Considerations

#### 6.1 Security
- **Input Validation**: Sanitize prompt arguments
- **Command Injection**: Use proper argument passing (avoid shell execution)
- **Path Validation**: Validate gemini-cli command path
- **Resource Limits**: Implement timeout and output size limits

#### 6.2 Performance
- **Async Execution**: Non-blocking command execution
- **Timeout Management**: Reasonable defaults with configurability
- **Memory Management**: Handle large outputs efficiently
- **Error Caching**: Avoid repeated failures for invalid commands

## Implementation Phases Summary

1. **Setup Phase**: Update dependencies, add CLI parsing
2. **Core Implementation**: Add prompt_gemini tool with basic functionality
3. **Integration Phase**: Update server initialization and tool registration
4. **Testing Phase**: Comprehensive testing suite
5. **Documentation Phase**: Update tool descriptions and examples
6. **Validation Phase**: End-to-end testing with real gemini-cli

## Success Criteria

- [ ] MCP server accepts `--gemini-cli-command` argument
- [ ] MCP server reads `GEMINI_CLI_COMMAND` environment variable
- [ ] New `prompt-gemini` tool executes successfully
- [ ] Tool returns gemini-cli responses properly formatted
- [ ] Error handling works for all failure scenarios
- [ ] All existing FileSystem tools continue to work
- [ ] Comprehensive test coverage
- [ ] Integration with MCP Inspector works correctly

## Risk Mitigation

- **Dependency Conflicts**: Pin specific versions, test compatibility
- **Command Execution Failures**: Robust error handling and user feedback
- **Performance Issues**: Implement timeouts and resource limits
- **Security Vulnerabilities**: Input validation and safe command execution
- **Backward Compatibility**: Maintain existing API contracts

This plan provides a structured approach to implementing the `prompt-gemini` tool while maintaining code quality, security, and backward compatibility.