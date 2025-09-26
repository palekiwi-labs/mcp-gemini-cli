use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    service::RequestContext,
    tool, tool_handler, tool_router,
};
use serde::Deserialize;
use tokio::process::Command;

// Allow dead code for JSON schema structs - they define complete API schemas for future extensibility
#[allow(dead_code)]

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PromptGeminiArgs {
    /// The prompt to send to Gemini CLI
    pub prompt: String,
    /// Output format: "json" or "text" (default)
    #[serde(default)]
    pub output_format: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct GeminiJsonResponse {
    pub response: String,
    pub stats: Option<GeminiStats>,
    pub error: Option<GeminiErrorResponse>,
}

#[derive(Debug, Deserialize)]
pub struct GeminiErrorResponse {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
    pub code: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct GeminiStats {
    pub models: Option<std::collections::HashMap<String, ModelStats>>,
    pub tools: Option<ToolStats>,
    pub files: Option<FileStats>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ModelStats {
    pub api: Option<ApiStats>,
    pub tokens: Option<TokenStats>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ApiStats {
    #[serde(rename = "totalRequests")]
    pub total_requests: Option<i32>,
    #[serde(rename = "totalErrors")]
    pub total_errors: Option<i32>,
    #[serde(rename = "totalLatencyMs")]
    pub total_latency_ms: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct TokenStats {
    pub prompt: Option<i32>,
    pub candidates: Option<i32>,
    pub total: Option<i32>,
    pub cached: Option<i32>,
    pub thoughts: Option<i32>,
    pub tool: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ToolStats {
    #[serde(rename = "totalCalls")]
    pub total_calls: Option<i32>,
    #[serde(rename = "totalSuccess")]
    pub total_success: Option<i32>,
    #[serde(rename = "totalFail")]
    pub total_fail: Option<i32>,
    #[serde(rename = "totalDurationMs")]
    pub total_duration_ms: Option<i32>,
    #[serde(rename = "totalDecisions")]
    pub total_decisions: Option<DecisionStats>,
    #[serde(rename = "byName")]
    pub by_name: Option<std::collections::HashMap<String, ToolDetailStats>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct DecisionStats {
    pub accept: Option<i32>,
    pub reject: Option<i32>,
    pub modify: Option<i32>,
    pub auto_accept: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ToolDetailStats {
    pub count: Option<i32>,
    pub success: Option<i32>,
    pub fail: Option<i32>,
    #[serde(rename = "durationMs")]
    pub duration_ms: Option<i32>,
    pub decisions: Option<DecisionStats>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct FileStats {
    #[serde(rename = "totalLinesAdded")]
    pub total_lines_added: Option<i32>,
    #[serde(rename = "totalLinesRemoved")]
    pub total_lines_removed: Option<i32>,
}

#[derive(Debug)]
enum ParseResult {
    JsonSuccess(Box<GeminiJsonResponse>),
    TextFallback(String),
    ParseError { raw_output: String, error: String },
}

fn parse_gemini_output(raw_output: &str, expect_json: bool) -> ParseResult {
    if expect_json {
        // Try multiple JSON parsing strategies
        if let Ok(json_response) = serde_json::from_str::<GeminiJsonResponse>(raw_output.trim()) {
            return ParseResult::JsonSuccess(Box::new(json_response));
        }

        // Try extracting JSON objects from mixed content
        if let Some(json_str) = extract_json_from_mixed_content(raw_output) 
            && let Ok(json_response) = serde_json::from_str::<GeminiJsonResponse>(&json_str) {
            return ParseResult::JsonSuccess(Box::new(json_response));
        }

        // Try line-by-line parsing
        for line in raw_output.lines() {
            let line = line.trim();
            if line.starts_with('{') 
                && let Ok(json_response) = serde_json::from_str::<GeminiJsonResponse>(line) {
                return ParseResult::JsonSuccess(Box::new(json_response));
            }
        }

        // JSON was expected but parsing failed
        ParseResult::ParseError {
            raw_output: raw_output.to_string(),
            error: "Failed to parse JSON from gemini CLI output".to_string(),
        }
    } else {
        // Plain text mode
        ParseResult::TextFallback(raw_output.to_string())
    }
}

fn extract_json_from_mixed_content(content: &str) -> Option<String> {
    // Look for JSON objects in mixed content
    let mut brace_count = 0;
    let mut start_pos = None;

    for (i, ch) in content.char_indices() {
        match ch {
            '{' => {
                if brace_count == 0 {
                    start_pos = Some(i);
                }
                brace_count += 1;
            }
            '}' => {
                brace_count -= 1;
                if brace_count == 0 {
                    if let Some(start) = start_pos {
                        let json_str = &content[start..=i];
                        // Validate it's actually JSON
                        if serde_json::from_str::<serde_json::Value>(json_str).is_ok() {
                            return Some(json_str.to_string());
                        }
                    }
                }
            }
            _ => {}
        }
    }
    None
}

#[derive(Clone)]
pub struct GeminiCli {
    tool_router: ToolRouter<GeminiCli>,
    gemini_cli_command: String,
    workspace: Option<String>,
}

#[tool_router]
impl GeminiCli {
    pub fn new(gemini_cli_command: String, workspace: Option<String>) -> Self {
        Self {
            tool_router: Self::tool_router(),
            gemini_cli_command,
            workspace,
        }
    }

    #[tool(
        name = "prompt-gemini",
        description = "Send a prompt to Gemini CLI and return the response"
    )]
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
            cmd.arg("--")
                .arg("--yolo")
                .arg("--prompt")
                .arg(&args.prompt);
        } else {
            // For other commands, use --prompt flag directly
            cmd.arg("--yolo").arg("--prompt").arg(&args.prompt);
        }

        // Add output format flag if JSON is requested
        if let Some(ref format) = args.output_format {
            if format == "json" {
                cmd.arg("--output-format").arg("json");
            }
        }

        // Use workspace from struct, falling back to environment variable
        let workspace = self
            .workspace
            .as_ref()
            .cloned()
            .or_else(|| std::env::var("GEMINI_WORKSPACE").ok());

        if let Some(ws) = workspace {
            cmd.env("GEMINI_WORKSPACE", ws);
        }

        let output = cmd.output().await;

        match output {
            Ok(output) => {
                if output.status.success() {
                    // Convert output to string, handling potential UTF-8 issues
                    let raw_response = String::from_utf8_lossy(&output.stdout);
                    let raw_response = raw_response.trim();

                    if raw_response.is_empty() {
                        return Ok(CallToolResult::success(vec![Content::text(
                            "Gemini CLI returned empty response".to_string(),
                        )]));
                    }

                    // Determine if JSON output was requested
                    let expect_json = args
                        .output_format
                        .as_ref()
                        .map(|f| f == "json")
                        .unwrap_or(false);

                    // Parse response using appropriate strategy
                    match parse_gemini_output(raw_response, expect_json) {
                        ParseResult::JsonSuccess(json_response) => {
                            // Check if there's an error in the JSON response
                            if let Some(error) = json_response.error {
                                return Err(McpError::internal_error(
                                    "gemini_api_error",
                                    Some(serde_json::json!({
                                        "error_type": error.error_type,
                                        "message": error.message,
                                        "code": error.code,
                                        "prompt": args.prompt
                                    })),
                                ));
                            }

                            // Return the response content
                            Ok(CallToolResult::success(vec![Content::text(
                                json_response.response,
                            )]))
                        }
                        ParseResult::TextFallback(text) => {
                            // Return raw response as plain text
                            Ok(CallToolResult::success(vec![Content::text(text)]))
                        }
                        ParseResult::ParseError { raw_output, error } => {
                            // JSON was expected but parsing failed, return error with raw output for debugging
                            Err(McpError::internal_error(
                                "gemini_json_parse_error",
                                Some(serde_json::json!({
                                    "parse_error": error,
                                    "raw_output": raw_output,
                                    "prompt": args.prompt
                                })),
                            ))
                        }
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
                "This server provides Gemini CLI integration with JSON output support. \
                Tools: prompt_gemini (send prompts to Gemini CLI with optional JSON format)."
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
        let gemini_cli = GeminiCli::new("nonexistent_command_12345".to_string(), None);
        let args = PromptGeminiArgs {
            prompt: "test prompt".to_string(),
            output_format: None,
        };

        let result = gemini_cli.prompt_gemini(Parameters(args)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_prompt_gemini_with_echo() {
        // Use echo command to simulate successful gemini CLI execution
        // Echo will output plain text, which should be returned successfully
        let gemini_cli = GeminiCli::new("echo".to_string(), None);
        let args = PromptGeminiArgs {
            prompt: "test response".to_string(),
            output_format: None,
        };

        let result = gemini_cli.prompt_gemini(Parameters(args)).await;
        assert!(result.is_ok());

        // Should return the echo output as plain text
        if let Ok(call_result) = result {
            assert!(!call_result.content.is_empty());
        }
    }

    #[tokio::test]
    async fn test_gemini_cli_new() {
        let gemini_cli = GeminiCli::new("test_command".to_string(), None);
        assert_eq!(gemini_cli.gemini_cli_command, "test_command");
    }

    #[tokio::test]
    async fn test_prompt_gemini_with_multiword_command() {
        // Test with a multi-word command like "echo hello"
        // This should successfully return the plain text output
        let gemini_cli = GeminiCli::new("echo hello".to_string(), None);
        let args = PromptGeminiArgs {
            prompt: "world".to_string(),
            output_format: None,
        };

        let result = gemini_cli.prompt_gemini(Parameters(args)).await;
        assert!(result.is_ok());

        if let Ok(call_result) = result {
            assert!(!call_result.content.is_empty());
        }
    }

    #[tokio::test]
    async fn test_prompt_gemini_with_empty_output() {
        // Test with a simple command that we know will work (true does nothing but exit successfully)
        // This test verifies the plain text response handling
        let gemini_cli = GeminiCli::new("true".to_string(), None);
        let args = PromptGeminiArgs {
            prompt: "test prompt".to_string(),
            output_format: None,
        };

        let result = gemini_cli.prompt_gemini(Parameters(args)).await;

        // Since 'true' returns empty output, it should result in empty response content
        assert!(result.is_ok());

        if let Ok(call_result) = result {
            assert!(!call_result.content.is_empty());
            // Should contain empty response message
            if let RawContent::Text(text_content) = &call_result.content[0].raw {
                assert_eq!(text_content.text, "Gemini CLI returned empty response");
            }
        }
    }

    #[tokio::test]
    async fn test_prompt_gemini_with_text_output() {
        // Test with a command that returns plain text
        let gemini_cli = GeminiCli::new("echo 'Hello from Gemini'".to_string(), None);
        let args = PromptGeminiArgs {
            prompt: "test prompt".to_string(),
            output_format: None,
        };

        let result = gemini_cli.prompt_gemini(Parameters(args)).await;
        assert!(result.is_ok());

        if let Ok(call_result) = result {
            assert!(!call_result.content.is_empty());
            // Should contain the echoed text
            if let RawContent::Text(text_content) = &call_result.content[0].raw {
                assert!(text_content.text.contains("Hello from Gemini"));
            }
        }
    }

    // JSON parsing tests
    #[test]
    fn test_parse_clean_json() {
        let json_output = r#"{"response": "Hello world", "stats": null, "error": null}"#;
        let result = parse_gemini_output(json_output, true);
        match result {
            ParseResult::JsonSuccess(response) => {
                assert_eq!(response.response, "Hello world");
                assert!(response.stats.is_none());
                assert!(response.error.is_none());
            }
            _ => panic!("Expected JsonSuccess"),
        }
    }

    #[test]
    fn test_parse_mixed_content() {
        let mixed_output = r#"Loading model...
Generating response...
{"response": "Hello world", "stats": null, "error": null}
Done."#;
        let result = parse_gemini_output(mixed_output, true);
        match result {
            ParseResult::JsonSuccess(response) => {
                assert_eq!(response.response, "Hello world");
            }
            _ => panic!("Expected JsonSuccess from mixed content"),
        }
    }

    #[test]
    fn test_parse_error_response() {
        let error_json = r#"{"response": "", "error": {"type": "ApiError", "message": "Test error", "code": 400}, "stats": null}"#;
        let result = parse_gemini_output(error_json, true);
        match result {
            ParseResult::JsonSuccess(response) => {
                assert!(response.error.is_some());
                let error = response.error.unwrap();
                assert_eq!(error.error_type, "ApiError");
                assert_eq!(error.message, "Test error");
                assert_eq!(error.code, Some(400));
            }
            _ => panic!("Expected JsonSuccess with error"),
        }
    }

    #[test]
    fn test_parse_fallback_to_error() {
        let invalid_json = "This is not JSON at all";
        let result = parse_gemini_output(invalid_json, true);
        match result {
            ParseResult::ParseError { raw_output, error } => {
                assert_eq!(raw_output, invalid_json);
                assert!(error.contains("Failed to parse JSON"));
            }
            _ => panic!("Expected ParseError"),
        }
    }

    #[test]
    fn test_text_mode() {
        let text_output = "Hello world";
        let result = parse_gemini_output(text_output, false);
        match result {
            ParseResult::TextFallback(text) => {
                assert_eq!(text, "Hello world");
            }
            _ => panic!("Expected TextFallback"),
        }
    }

    #[test]
    fn test_extract_json_from_mixed_content() {
        let mixed_content = r#"Some log output
        {"response": "test", "error": null, "stats": null}
        More output"#;

        let json_str = extract_json_from_mixed_content(mixed_content);
        assert!(json_str.is_some());

        let json_obj: serde_json::Value = serde_json::from_str(&json_str.unwrap()).unwrap();
        assert_eq!(json_obj["response"], "test");
    }

    #[tokio::test]
    async fn test_prompt_gemini_with_json_output() {
        // Test JSON output mode with a command that returns valid JSON
        let valid_json =
            r#"{"response": "Paris is the capital of France", "error": null, "stats": null}"#;
        let gemini_cli = GeminiCli::new(format!("echo '{}'", valid_json), None);
        let args = PromptGeminiArgs {
            prompt: "What is the capital of France?".to_string(),
            output_format: Some("json".to_string()),
        };

        let result = gemini_cli.prompt_gemini(Parameters(args)).await;
        assert!(result.is_ok());

        if let Ok(call_result) = result {
            assert!(!call_result.content.is_empty());
            if let RawContent::Text(text_content) = &call_result.content[0].raw {
                assert_eq!(text_content.text, "Paris is the capital of France");
            }
        }
    }

    #[tokio::test]
    async fn test_prompt_gemini_with_json_error_response() {
        // Test JSON output mode with an error response
        let error_json = r#"{"response": "", "error": {"type": "AuthError", "message": "API key invalid", "code": 401}, "stats": null}"#;
        let gemini_cli = GeminiCli::new(format!("echo '{}'", error_json), None);
        let args = PromptGeminiArgs {
            prompt: "test".to_string(),
            output_format: Some("json".to_string()),
        };

        let result = gemini_cli.prompt_gemini(Parameters(args)).await;
        assert!(result.is_err());

        if let Err(error) = result {
            // Should be a gemini API error
            assert!(
                error.message.contains("gemini_api_error") || error.message.contains("AuthError")
            );
        }
    }

    #[tokio::test]
    async fn test_prompt_gemini_json_parse_fallback() {
        // Test JSON mode with invalid JSON (should fall back to parse error)
        let invalid_json = "This is not valid JSON";
        let gemini_cli = GeminiCli::new(format!("echo '{}'", invalid_json), None);
        let args = PromptGeminiArgs {
            prompt: "test".to_string(),
            output_format: Some("json".to_string()),
        };

        let result = gemini_cli.prompt_gemini(Parameters(args)).await;
        assert!(result.is_err());

        if let Err(error) = result {
            // Should be a JSON parse error
            assert!(error.message.contains("gemini_json_parse_error"));
        }
    }
}

