use std::path::Path;

use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    handler::server::{
        router::tool::ToolRouter,
        wrapper::Parameters,
    },
    model::*,
    service::RequestContext,
    tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};
use tokio::fs;

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
}

#[tool_router]
impl FileSystem {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
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
                        let file_type = if entry.file_type().await.map(|ft| ft.is_dir()).unwrap_or(false) {
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
            Ok(content) => {
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Content of '{}':\n\n{}",
                    args.path, content
                ))]))
            }
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
            Ok(()) => {
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Successfully wrote {} bytes to '{}'",
                    args.content.len(),
                    args.path
                ))]))
            }
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
}

#[tool_handler]
impl ServerHandler for FileSystem {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "This server provides filesystem tools for basic file and directory operations. \
                Tools: list_files (list directory contents), read_file (read file contents), \
                write_file (write to file), get_file_info (get file metadata).".to_string()
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