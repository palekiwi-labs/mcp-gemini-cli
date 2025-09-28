# JSON Output Implementation Plan for mcp-gemini-cli

## Overview

Based on analysis of commit `a998df4fc8b3a1207336ba6d8dd9620ed0a3d313`, JSON output functionality was previously implemented but reverted because it wasn't supported by gemini CLI at the time. Now that JSON output is supported (as documented in `ai_docs/gemini-cli/headless.md`), we can re-implement this feature with proper parsing to handle mixed stdout content.

## Problem Statement

The gemini CLI now supports `--output-format json` but may output logs and other content mixed with JSON output. We need to:
1. Parse the response from gemini and extract only the JSON
2. Handle mixed stdout containing logs and other content
3. Provide backward compatibility with existing plain text behavior
4. Handle error responses gracefully

## Implementation Strategy

### 1. JSON Parsing Strategy

**Problem**: Gemini CLI may output logs and other content mixed with JSON output.

**Solution**: Multi-layered parsing approach:
1. **Attempt full JSON parse** - try parsing entire stdout as JSON first
2. **Extract JSON objects** - scan for `{...}` patterns and attempt parsing  
3. **Line-by-line parsing** - look for lines starting with `{` and try parsing each
4. **Fallback to text** - if no valid JSON found, return as plain text with warning

### 2. Response Struct Definitions

Based on the documented schema from `ai_docs/gemini-cli/headless.md`:

```rust
#[derive(Debug, Deserialize)]
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
pub struct GeminiStats {
    pub models: Option<std::collections::HashMap<String, ModelStats>>,
    pub tools: Option<ToolStats>,
    pub files: Option<FileStats>,
}

#[derive(Debug, Deserialize)]
pub struct ModelStats {
    pub api: Option<ApiStats>,
    pub tokens: Option<TokenStats>,
}

#[derive(Debug, Deserialize)]
pub struct ApiStats {
    #[serde(rename = "totalRequests")]
    pub total_requests: Option<i32>,
    #[serde(rename = "totalErrors")]
    pub total_errors: Option<i32>,
    #[serde(rename = "totalLatencyMs")]
    pub total_latency_ms: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct TokenStats {
    pub prompt: Option<i32>,
    pub candidates: Option<i32>,
    pub total: Option<i32>,
    pub cached: Option<i32>,
    pub thoughts: Option<i32>,
    pub tool: Option<i32>,
}

#[derive(Debug, Deserialize)]
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
pub struct DecisionStats {
    pub accept: Option<i32>,
    pub reject: Option<i32>,
    pub modify: Option<i32>,
    pub auto_accept: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct ToolDetailStats {
    pub count: Option<i32>,
    pub success: Option<i32>,
    pub fail: Option<i32>,
    #[serde(rename = "durationMs")]
    pub duration_ms: Option<i32>,
    pub decisions: Option<DecisionStats>,
}

#[derive(Debug, Deserialize)]
pub struct FileStats {
    #[serde(rename = "totalLinesAdded")]
    pub total_lines_added: Option<i32>,
    #[serde(rename = "totalLinesRemoved")]
    pub total_lines_removed: Option<i32>,
}
```

### 3. Command Modification

**Changes needed**:
- Add optional `output_format` field to `PromptGeminiArgs`
- Conditionally add `--output-format json` flag to command construction
- Default to text format for backward compatibility

```rust
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PromptGeminiArgs {
    /// The prompt to send to Gemini CLI
    pub prompt: String,
    /// Output format: "json" or "text" (default)
    #[serde(default)]
    pub output_format: Option<String>,
}

fn default_output_format() -> Option<String> {
    None // Defaults to text format
}
```

### 4. Error Handling Strategy

**Strategy**:
- Try JSON parsing with multiple fallback approaches
- Preserve raw output in error cases for debugging
- Handle gemini CLI error responses gracefully
- Provide clear error messages for different failure modes

```rust
enum ParseResult {
    JsonSuccess(GeminiJsonResponse),
    TextFallback(String),
    ParseError { raw_output: String, error: String },
}

fn parse_gemini_output(raw_output: &str, expect_json: bool) -> ParseResult {
    if expect_json {
        // Try multiple JSON parsing strategies
        if let Ok(json_response) = serde_json::from_str::<GeminiJsonResponse>(raw_output.trim()) {
            return ParseResult::JsonSuccess(json_response);
        }
        
        // Try extracting JSON objects from mixed content
        if let Some(json_str) = extract_json_from_mixed_content(raw_output) {
            if let Ok(json_response) = serde_json::from_str::<GeminiJsonResponse>(&json_str) {
                return ParseResult::JsonSuccess(json_response);
            }
        }
        
        // Try line-by-line parsing
        for line in raw_output.lines() {
            let line = line.trim();
            if line.starts_with('{') {
                if let Ok(json_response) = serde_json::from_str::<GeminiJsonResponse>(line) {
                    return ParseResult::JsonSuccess(json_response);
                }
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
    let mut chars = content.char_indices();
    
    while let Some((i, ch)) = chars.next() {
        match ch {
            '{' => {
                if brace_count == 0 {
                    start_pos = Some(i);
                }
                brace_count += 1;
            }
            '}' => {
                brace_count -= 1;
                if brace_count == 0 && start_pos.is_some() {
                    let json_str = &content[start_pos.unwrap()..=i];
                    // Validate it's actually JSON
                    if serde_json::from_str::<serde_json::Value>(json_str).is_ok() {
                        return Some(json_str.to_string());
                    }
                }
            }
            _ => {}
        }
    }
    None
}
```

### 5. Testing Strategy

**Test cases needed**:
- Valid JSON responses with and without stats/errors  
- Mixed output (logs + JSON) parsing
- Invalid JSON handling and fallbacks
- Command construction with different output formats
- Error response parsing
- Empty response handling
- Backward compatibility with existing plain text behavior

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_clean_json() {
        let json_output = r#"{"response": "Hello world", "stats": null, "error": null}"#;
        let result = parse_gemini_output(json_output, true);
        match result {
            ParseResult::JsonSuccess(response) => {
                assert_eq!(response.response, "Hello world");
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
        let error_json = r#"{"error": {"type": "ApiError", "message": "Test error", "code": 400}}"#;
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
    fn test_fallback_to_text() {
        let invalid_json = "This is not JSON at all";
        let result = parse_gemini_output(invalid_json, true);
        match result {
            ParseResult::ParseError { raw_output, .. } => {
                assert_eq!(raw_output, invalid_json);
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
}
```

### 6. Backward Compatibility

**Strategy**: Make JSON output opt-in to avoid breaking existing integrations:
- Default behavior remains unchanged (plain text output)
- JSON output only enabled when `output_format: "json"` specified
- All existing tests continue to pass without modification
- New functionality available without breaking changes

## Implementation Steps

1. **Add new struct definitions** for JSON response types
2. **Update PromptGeminiArgs** to include optional output_format field
3. **Implement robust JSON parsing function** with multiple fallback strategies
4. **Modify command construction** to conditionally add --output-format flag
5. **Update response processing logic** in prompt_gemini method
6. **Add comprehensive tests** for all scenarios
7. **Update tool description** to document new JSON output capability

## Key Benefits

- **Clean JSON parsing** from mixed stdout content
- **Preserved backward compatibility** with existing integrations
- **Rich error handling** with fallback strategies
- **Comprehensive stats access** for advanced use cases
- **Robust parsing** handles various output scenarios

## Usage Examples

### Basic JSON Output
```rust
let args = PromptGeminiArgs {
    prompt: "What is the capital of France?".to_string(),
    output_format: Some("json".to_string()),
};
```

### Backward Compatible (Default Text)
```rust
let args = PromptGeminiArgs {
    prompt: "What is the capital of France?".to_string(),
    output_format: None, // Defaults to text
};
```

## Files to Modify

- `src/tools.rs` - Main implementation
- `src/tools.rs` (tests section) - Add comprehensive tests
- Tool description in `get_info()` method

This implementation follows the exact schema documented in `ai_docs/gemini-cli/headless.md`, ensuring full compatibility with the `--output-format json` flag that's now supported by gemini CLI.
