# RMCP - Rust SDK for Model Context Protocol

**Version:** 0.7.0  
**Source:** [docs.rs/rmcp/latest/rmcp](https://docs.rs/rmcp/latest/rmcp/)  
**Repository:** [github.com/modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk)  
**License:** MIT  

## Summary

The official Rust SDK for the Model Context Protocol (MCP).

The MCP is a protocol that allows AI assistants to communicate with other services. `rmcp` is the official Rust implementation of this protocol.

There are two ways in which the library can be used, namely to build a server or to build a client.

## Server

A server is a service that exposes capabilities. For example, a common use-case is for the server to make multiple tools available to clients such as Claude Desktop or the Cursor IDE.

For example, to implement a server that has a tool that can count, you would make an object for that tool and add an implementation with the `#[tool_router]` macro:

```rust
use std::sync::Arc;
use rmcp::{ErrorData as McpError, model::*, tool, tool_router, handler::server::tool::ToolRouter};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct Counter {
    counter: Arc<Mutex<i32>>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl Counter {
    fn new() -> Self {
        Self {
            counter: Arc::new(Mutex::new(0)),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Increment the counter by 1")]
    async fn increment(&self) -> Result<CallToolResult, McpError> {
        let mut counter = self.counter.lock().await;
        *counter += 1;
        Ok(CallToolResult::success(vec![Content::text(
            counter.to_string(),
        )]))
    }
}
```

### Structured Output

Tools can also return structured JSON data with schemas. Use the `Json` wrapper:

```rust
#[derive(Serialize, Deserialize, JsonSchema)]
struct CalculationRequest {
    a: i32,
    b: i32,
    operation: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
struct CalculationResult {
    result: i32,
    operation: String,
}

#[tool(name = "calculate", description = "Perform a calculation")]
async fn calculate(&self, params: Parameters<CalculationRequest>) -> Result<Json<CalculationResult>, String> {
    let result = match params.0.operation.as_str() {
        "add" => params.0.a + params.0.b,
        "multiply" => params.0.a * params.0.b,
        _ => return Err("Unknown operation".to_string()),
    };
     
    Ok(Json(CalculationResult { result, operation: params.0.operation }))
}
```

The `#[tool]` macro automatically generates an output schema from the `CalculationResult` type.

Next also implement `ServerHandler` for your server type and start the server inside `main` by calling `.serve(...)`. See the examples directory in the repository for more information.

## Client

A client can be used to interact with a server. Clients can be used to get a list of the available tools and to call them. For example, we can `uv` to start a MCP server in Python and then list the tools and call `git status` as follows:

```rust
use anyhow::Result;
use rmcp::{model::CallToolRequestParam, service::ServiceExt, transport::{TokioChildProcess, ConfigureCommandExt}};
use tokio::process::Command;

async fn client() -> Result<()> {
    let service = ().serve(TokioChildProcess::new(Command::new("uvx").configure(|cmd| {
        cmd.arg("mcp-server-git");
    }))?).await?;

    // Initialize
    let server_info = service.peer_info();
    println!("Connected to server: {server_info:#?}");

    // List tools
    let tools = service.list_tools(Default::default()).await?;
    println!("Available tools: {tools:#?}");

    // Call tool 'git_status' with arguments = {"repo_path": "."}
    let tool_result = service
        .call_tool(CallToolRequestParam {
            name: "git_status".into(),
            arguments: serde_json::json!({ "repo_path": "." }).as_object().cloned(),
        })
        .await?;
    println!("Tool result: {tool_result:#?}");

    service.cancel().await?;
    Ok(())
}
```

## Re-exports

- `pub use error::ErrorData;`
- `pub use handler::client::ClientHandler;` (client feature)
- `pub use handler::server::ServerHandler;` (server feature)
- `pub use handler::server::wrapper::Json;` (server feature)
- `pub use service::Peer;` (client or server feature)
- `pub use service::Service;` (client or server feature)
- `pub use service::ServiceError;` (client or server feature)
- `pub use service::ServiceExt;` (client or server feature)
- `pub use service::RoleClient;` (client feature)
- `pub use service::serve_client;` (client feature)
- `pub use service::RoleServer;` (server feature)
- `pub use service::serve_server;` (server feature)
- `pub use schemars;` (macros and server features)
- `pub use serde;` (macros feature)
- `pub use serde_json;` (macros feature)

## Modules

- **handler** - Handler implementations
- **model** - Basic data types in MCP specification
- **service** - Service layer (client or server feature)
- **transport** - Transport layer

## Macros

- **const_string** - String constant macro
- **elicit_safe** - Macro to mark types as safe for elicitation by verifying they generate object schemas (server and client/server features)
- **object** - Use this macro just like `serde_json::json!` (macros feature)
- **paste** - Paste macro (macros and server features)

## Enums

- **RmcpError** - Unified error type for the errors that could be returned by the service

## Type Aliases

- **Error** - *(Deprecated)*

## Attribute Macros

- **prompt** - Prompt attribute macro (macros and server features)
- **prompt_handler** - Prompt handler attribute macro (macros and server features)
- **prompt_router** - Prompt router attribute macro (macros and server features)
- **tool** - Tool attribute macro (macros and server features)
- **tool_handler** - Tool handler attribute macro (macros and server features)
- **tool_router** - Tool router attribute macro (macros and server features)

## Dependencies

### Required Dependencies
- **futures** ^0.3
- **pin-project-lite** ^0.2
- **serde** ^1.0
- **serde_json** ^1.0
- **thiserror** ^2
- **tokio** ^1
- **tokio-util** ^0.7
- **tracing** ^0.1
- **chrono** ^0.4.38

### Optional Dependencies
- **axum** ^0.8 (optional)
- **base64** ^0.22 (optional)
- **bytes** ^1 (optional)
- **http** ^1 (optional)
- **http-body** ^1 (optional)
- **http-body-util** ^0.1 (optional)
- **oauth2** ^5.0 (optional)
- **paste** ^1 (optional)
- **process-wrap** ^8.2 (optional)
- **rand** ^0.9 (optional)
- **reqwest** ^0.12 (optional)
- **rmcp-macros** ^0.7.0 (optional)
- **schemars** ^1.0 (optional)
- **sse-stream** ^0.2 (optional)
- **tokio-stream** ^0.1 (optional)
- **tower-service** ^0.3 (optional)
- **url** ^2.4 (optional)
- **uuid** ^1 (optional)

### Development Dependencies
- **anyhow** ^1.0
- **async-trait** ^0.1
- **schemars** ^1.0
- **tokio** ^1
- **tracing-subscriber** ^0.3

## Documentation Coverage

**27.27%** of the crate is documented.

## Package Information

- **Crate page:** [crates.io/crates/rmcp](https://crates.io/crates/rmcp)
- **Homepage:** [github.com/modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk)
- **Repository:** [github.com/modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk/)

### Owners
- [4t145](https://crates.io/users/4t145)
- [jokemanfire](https://crates.io/users/jokemanfire)
- [alexhancock](https://crates.io/users/alexhancock)

### Supported Platforms
- i686-unknown-linux-gnu
- x86_64-unknown-linux-gnu