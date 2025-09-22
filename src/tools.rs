use std::path::Path;

use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    service::RequestContext,
    tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};
use tokio::{fs, process::Command};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListFilesArgs {
    /// Path to directory to list (defaults to current directory)
    #[serde(default = "default_current_dir")]
    pub path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ReadFileArgs {
    /// Path to the file to read
    pub path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WriteFileArgs {
    /// Path to the file to write
    pub path: String,
    /// Content to write to the file
    pub content: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FileInfoArgs {
    /// Path to get information about
    pub path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PromptGeminiArgs {
    /// The prompt to send to Gemini CLI
    pub prompt: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct FileInfo {
    pub path: String,
    pub size: u64,
    pub is_file: bool,
    pub is_dir: bool,
    pub modified: String,
}

fn default_current_dir() -> String {
    ".".to_string()
}

#[derive(Clone)]
pub struct FileSystem {
    tool_router: ToolRouter<FileSystem>,
    gemini_cli_command: String,
}

#[tool_router]
impl FileSystem {
    pub fn new(gemini_cli_command: String) -> Self {
        Self {
            tool_router: Self::tool_router(),
            gemini_cli_command,
        }
    }

    #[tool(description = "List files and directories in a given path")]
    async fn list_files(
        &self,
        Parameters(args): Parameters<ListFilesArgs>,
    ) -> Result<CallToolResult, McpError> {
        let path = Path::new(&args.path);

        match fs::read_dir(path).await {
            Ok(mut entries) => {
                let mut files = Vec::new();

                while let Ok(Some(entry)) = entries.next_entry().await {
                    if let Ok(file_name) = entry.file_name().into_string() {
                        let file_type = if entry
                            .file_type()
                            .await
                            .map(|ft| ft.is_dir())
                            .unwrap_or(false)
                        {
                            "[DIR]"
                        } else {
                            "[FILE]"
                        };
                        files.push(format!("{} {}", file_type, file_name));
                    }
                }

                files.sort();
                let content = if files.is_empty() {
                    "Directory is empty".to_string()
                } else {
                    format!("Contents of '{}':\n{}", args.path, files.join("\n"))
                };

                Ok(CallToolResult::success(vec![Content::text(content)]))
            }
            Err(e) => Err(McpError::internal_error(
                "failed_to_read_directory",
                Some(serde_json::json!({
                    "path": args.path,
                    "error": e.to_string()
                })),
            )),
        }
    }

    #[tool(description = "Read the contents of a file")]
    async fn read_file(
        &self,
        Parameters(args): Parameters<ReadFileArgs>,
    ) -> Result<CallToolResult, McpError> {
        match fs::read_to_string(&args.path).await {
            Ok(content) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Content of '{}':\n\n{}",
                args.path, content
            ))])),
            Err(e) => Err(McpError::internal_error(
                "failed_to_read_file",
                Some(serde_json::json!({
                    "path": args.path,
                    "error": e.to_string()
                })),
            )),
        }
    }

    #[tool(description = "Write content to a file")]
    async fn write_file(
        &self,
        Parameters(args): Parameters<WriteFileArgs>,
    ) -> Result<CallToolResult, McpError> {
        match fs::write(&args.path, &args.content).await {
            Ok(()) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Successfully wrote {} bytes to '{}'",
                args.content.len(),
                args.path
            ))])),
            Err(e) => Err(McpError::internal_error(
                "failed_to_write_file",
                Some(serde_json::json!({
                    "path": args.path,
                    "error": e.to_string()
                })),
            )),
        }
    }

    #[tool(description = "Get information about a file or directory")]
    async fn get_file_info(
        &self,
        Parameters(args): Parameters<FileInfoArgs>,
    ) -> Result<CallToolResult, McpError> {
        let path = Path::new(&args.path);

        match fs::metadata(path).await {
            Ok(metadata) => {
                let modified = metadata
                    .modified()
                    .map(|time| {
                        use std::time::UNIX_EPOCH;
                        let duration = time.duration_since(UNIX_EPOCH).unwrap_or_default();
                        format!("{} seconds since epoch", duration.as_secs())
                    })
                    .unwrap_or_else(|_| "unknown".to_string());

                let info = FileInfo {
                    path: args.path.clone(),
                    size: metadata.len(),
                    is_file: metadata.is_file(),
                    is_dir: metadata.is_dir(),
                    modified,
                };

                let content = format!(
                    "File info for '{}':\nSize: {} bytes\nType: {}\nModified: {}",
                    info.path,
                    info.size,
                    if info.is_dir { "Directory" } else { "File" },
                    info.modified
                );

                Ok(CallToolResult::success(vec![Content::text(content)]))
            }
            Err(e) => Err(McpError::internal_error(
                "failed_to_get_file_info",
                Some(serde_json::json!({
                    "path": args.path,
                    "error": e.to_string()
                })),
            )),
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
impl ServerHandler for FileSystem {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "This server provides filesystem tools and Gemini CLI integration. \
                Tools: list_files (list directory contents), read_file (read file contents), \
                write_file (write to file), get_file_info (get file metadata), \
                prompt_gemini (send prompts to Gemini CLI)."
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
        let fs = FileSystem::new("nonexistent_command_12345".to_string());
        let args = PromptGeminiArgs {
            prompt: "test prompt".to_string(),
        };

        let result = fs.prompt_gemini(Parameters(args)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_prompt_gemini_with_echo() {
        // Use echo command to simulate successful gemini CLI execution
        let fs = FileSystem::new("echo".to_string());
        let args = PromptGeminiArgs {
            prompt: "test response".to_string(),
        };

        let result = fs.prompt_gemini(Parameters(args)).await;
        assert!(result.is_ok());

        if let Ok(call_result) = result {
            // Echo will return "--prompt test response" since we're passing those as args
            assert!(!call_result.content.is_empty());
        }
    }

    #[tokio::test]
    async fn test_filesystem_new() {
        let fs = FileSystem::new("test_command".to_string());
        assert_eq!(fs.gemini_cli_command, "test_command");
    }

    #[tokio::test]
    async fn test_prompt_gemini_with_multiword_command() {
        // Test with a multi-word command like "echo hello"
        let fs = FileSystem::new("echo hello".to_string());
        let args = PromptGeminiArgs {
            prompt: "world".to_string(),
        };
        
        let result = fs.prompt_gemini(Parameters(args)).await;
        assert!(result.is_ok());
        
        if let Ok(call_result) = result {
            // Should contain "hello --prompt world"
            assert!(!call_result.content.is_empty());
        }
    }
}

