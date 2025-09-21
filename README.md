# MCP SSE Server Template

A [Nix flake](https://nixos.wiki/wiki/Flakes) template for building Model Context Protocol (MCP) servers with Server-Sent Events (SSE) transport in Rust.

## Features

- **Rust 1.85** with complete toolchain via [fenix](https://github.com/nix-community/fenix)
- **SSE Transport** for web-compatible real-time communication  
- **Filesystem Tools** demonstrating practical MCP tool development
- **Complete Dev Environment** with rust-analyzer, cargo-watch, and more
- **Ready-to-run** minimal but functional server

## Quick Start

```bash
# Create a new project from this template
nix flake init -t github:your-org/flake-templates#rust-mcp-server

# Enter the development environment
nix develop

# Start the MCP SSE server
cargo run
```

The server will start on `http://127.0.0.1:8000` with:
- SSE endpoint: `/sse`  
- Message endpoint: `/message`

## Available Tools

This template includes filesystem operation tools:

- **`list_files`** - List files and directories in a path
- **`read_file`** - Read the contents of a file
- **`write_file`** - Write content to a file  
- **`get_file_info`** - Get metadata about a file or directory

## Testing

### With MCP Inspector

The easiest way to test your server is with the [MCP Inspector](https://github.com/modelcontextprotocol/inspector):

1. Start your server: `cargo run`
2. Open the MCP Inspector in your browser
3. Connect to: `http://127.0.0.1:8000/sse`

### With curl

Test the SSE connection:
```bash
curl -N -H "Accept: text/event-stream" http://127.0.0.1:8000/sse
```

Send a message:
```bash
curl -X POST http://127.0.0.1:8000/message \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {}}'
```

## Development

### Project Structure

```
src/
├── main.rs          # SSE server entry point
└── tools.rs         # Filesystem tools implementation
```

### Adding New Tools

1. Add your tool function to `src/tools.rs`:

```rust
#[tool(description = "Your tool description")]
async fn your_tool(
    &self,
    Parameters(args): Parameters<YourArgs>,
) -> Result<CallToolResult, McpError> {
    // Implementation
    Ok(CallToolResult::success(vec![Content::text("result")]))
}
```

2. Define argument structs with JSON schema:

```rust
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct YourArgs {
    /// Description of the parameter
    pub param: String,
}
```

### Development Commands

```bash
# Auto-restart on file changes
cargo watch -x run

# Check code formatting
cargo fmt --check

# Run lints
cargo clippy

# Expand macros (useful for debugging)
cargo expand
```

## Architecture

This template uses:

- **[rmcp](https://crates.io/crates/rmcp)** - Official Rust MCP SDK
- **SSE Transport** - HTTP-based real-time communication
- **Tool Router** - Macro-based tool registration and routing
- **JSON Schema** - Automatic parameter validation via [schemars](https://crates.io/crates/schemars)
- **Axum** - Modern async web framework
- **Tokio** - Async runtime

## Resources

- [MCP Specification](https://spec.modelcontextprotocol.io/specification/2024-11-05/)
- [Rust MCP SDK](https://github.com/modelcontextprotocol/rust-sdk) 
- [MCP Inspector](https://github.com/modelcontextprotocol/inspector)
- [MCP Examples](https://github.com/modelcontextprotocol/rust-sdk/tree/main/examples)

## Next Steps

1. **Customize the tools** in `src/tools.rs` for your use case
2. **Add authentication** if needed (see [auth examples](https://github.com/modelcontextprotocol/rust-sdk/tree/main/examples/servers/src))
3. **Add prompts** for LLM interaction (see [prompt examples](https://github.com/modelcontextprotocol/rust-sdk/blob/main/examples/servers/src/prompt_stdio.rs))
4. **Add resources** for data access (see [counter example](https://github.com/modelcontextprotocol/rust-sdk/blob/main/examples/servers/src/common/counter.rs))

Happy building!