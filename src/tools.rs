use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    service::RequestContext,
    tool, tool_handler, tool_router,
};
use serde::Deserialize;
use tokio::process::Command;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PromptGeminiArgs {
    /// The prompt to send to Gemini CLI
    pub prompt: String,
}

#[derive(Clone)]
pub struct GeminiCli {
    tool_router: ToolRouter<GeminiCli>,
    gemini_cli_command: String,
}

#[tool_router]
impl GeminiCli {
    pub fn new(gemini_cli_command: String) -> Self {
        Self {
            tool_router: Self::tool_router(),
            gemini_cli_command,
        }
    }

    #[tool(description = "Send a prompt to Gemini CLI and return the response")]
    async fn prompt_gemini(
        &self,
        Parameters(args): Parameters<PromptGeminiArgs>,
    ) -> Result<CallToolResult, McpError> {
        // Parse command string to handle commands with arguments (e.g., "task ai:run")
        let parts: Vec<&str> = self.gemini_cli_command.split_whitespace().collect();
        if parts.is_empty() {
            return Err(McpError::internal_error(
                "empty_gemini_command",
                Some(serde_json::json!({
                    "error": "Gemini CLI command is empty"
                })),
            ));
        }

        // Execute gemini-cli command
        let mut cmd = Command::new(parts[0]);
        if parts.len() > 1 {
            cmd.args(&parts[1..]);
        }

        // For task runner, use -- separator to pass CLI args
        if parts[0] == "task" {
            cmd.arg("--").arg("--prompt").arg(&args.prompt);
        } else {
            // For other commands, use --prompt flag directly
            cmd.arg("--prompt").arg(&args.prompt);
        }
        
        let output = cmd.output().await;

        match output {
            Ok(output) => {
                if output.status.success() {
                    // Convert output to string, handling potential UTF-8 issues
                    let response = String::from_utf8_lossy(&output.stdout);
                    let response = response.trim();

                    if response.is_empty() {
                        Ok(CallToolResult::success(vec![Content::text(
                            "Gemini CLI returned empty response".to_string(),
                        )]))
                    } else {
                        Ok(CallToolResult::success(vec![Content::text(
                            response.to_string(),
                        )]))
                    }
                } else {
                    // Handle non-zero exit code
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let _error_msg = if stderr.trim().is_empty() {
                        format!(
                            "Gemini CLI exited with code {}",
                            output.status.code().unwrap_or(-1)
                        )
                    } else {
                        format!("Gemini CLI error: {}", stderr.trim())
                    };

                    Err(McpError::internal_error(
                        "gemini_cli_execution_failed",
                        Some(serde_json::json!({
                            "exit_code": output.status.code(),
                            "stderr": stderr.trim(),
                            "prompt": args.prompt
                        })),
                    ))
                }
            }
            Err(e) => {
                // Handle command execution failure (e.g., command not found)
                let _error_msg = if e.kind() == std::io::ErrorKind::NotFound {
                    format!(
                        "Gemini CLI command '{}' not found. Please ensure it's installed and accessible.",
                        self.gemini_cli_command
                    )
                } else {
                    format!("Failed to execute Gemini CLI: {}", e)
                };

                Err(McpError::internal_error(
                    "gemini_cli_command_failed",
                    Some(serde_json::json!({
                        "command": self.gemini_cli_command,
                        "error": e.to_string(),
                        "prompt": args.prompt
                    })),
                ))
            }
        }
    }
}

#[tool_handler]
impl ServerHandler for GeminiCli {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "This server provides Gemini CLI integration. \
                Tools: prompt_gemini (send prompts to Gemini CLI)."
                    .to_string(),
            ),
        }
    }

    async fn initialize(
        &self,
        _request: InitializeRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        Ok(self.get_info())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::handler::server::wrapper::Parameters;

    #[tokio::test]
    async fn test_prompt_gemini_command_not_found() {
        let gemini_cli = GeminiCli::new("nonexistent_command_12345".to_string());
        let args = PromptGeminiArgs {
            prompt: "test prompt".to_string(),
        };

        let result = gemini_cli.prompt_gemini(Parameters(args)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_prompt_gemini_with_echo() {
        // Use echo command to simulate successful gemini CLI execution
        let gemini_cli = GeminiCli::new("echo".to_string());
        let args = PromptGeminiArgs {
            prompt: "test response".to_string(),
        };

        let result = gemini_cli.prompt_gemini(Parameters(args)).await;
        assert!(result.is_ok());

        if let Ok(call_result) = result {
            // Echo will return "--prompt test response" since we're passing those as args
            assert!(!call_result.content.is_empty());
        }
    }

    #[tokio::test]
    async fn test_gemini_cli_new() {
        let gemini_cli = GeminiCli::new("test_command".to_string());
        assert_eq!(gemini_cli.gemini_cli_command, "test_command");
    }

    #[tokio::test]
    async fn test_prompt_gemini_with_multiword_command() {
        // Test with a multi-word command like "echo hello"
        let gemini_cli = GeminiCli::new("echo hello".to_string());
        let args = PromptGeminiArgs {
            prompt: "world".to_string(),
        };
        
        let result = gemini_cli.prompt_gemini(Parameters(args)).await;
        assert!(result.is_ok());
        
        if let Ok(call_result) = result {
            // Should contain "hello --prompt world"
            assert!(!call_result.content.is_empty());
        }
    }
}