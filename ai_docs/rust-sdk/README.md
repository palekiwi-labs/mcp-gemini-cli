# RMCP

[![Crates.io Version](https://img.shields.io/crates/v/rmcp)](https://crates.io/crates/rmcp)
[![Coverage](https://github.com/modelcontextprotocol/rust-sdk/raw/main/docs/coverage.svg)](https://github.com/modelcontextprotocol/rust-sdk/blob/main/docs/coverage.svg)

An official Rust Model Context Protocol SDK implementation with tokio async runtime.

This repository contains the following crates:

- [rmcp](https://github.com/modelcontextprotocol/rust-sdk/blob/main/crates/rmcp): The core crate providing the RMCP protocol implementation (If you want to get more information, please visit [rmcp](https://github.com/modelcontextprotocol/rust-sdk/blob/main/crates/rmcp/README.md))
- [rmcp-macros](https://github.com/modelcontextprotocol/rust-sdk/blob/main/crates/rmcp-macros): A procedural macro crate for generating RMCP tool implementations (If you want to get more information, please visit [rmcp-macros](https://github.com/modelcontextprotocol/rust-sdk/blob/main/crates/rmcp-macros/README.md))

## Usage

### Import the crate

```toml
rmcp = { version = "0.2.0", features = ["server"] }
## or dev channel
rmcp = { git = "https://github.com/modelcontextprotocol/rust-sdk", branch = "main" }
```

### Third Dependencies

Basic dependencies:

- [tokio required](https://github.com/tokio-rs/tokio)
- [serde required](https://github.com/serde-rs/serde)

### Build a Client

Start a client

```rust
use rmcp::{ServiceExt, transport::{TokioChildProcess, ConfigureCommandExt}};
use tokio::process::Command;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ().serve(TokioChildProcess::new(Command::new("npx").configure(|cmd| {
        cmd.arg("-y").arg("@modelcontextprotocol/server-everything");
    }))?).await?;
    Ok(())
}
```

### Build a Server

Build a transport

```rust
use tokio::io::{stdin, stdout};
let transport = (stdin(), stdout());
```

Build a service

You can easily build a service by using [`ServerHandler`](https://github.com/modelcontextprotocol/rust-sdk/blob/main/crates/rmcp/src/handler/server.rs) or [`ClientHandler`](https://github.com/modelcontextprotocol/rust-sdk/blob/main/crates/rmcp/src/handler/client.rs).

```rust
let service = common::counter::Counter::new();
```

Start the server

```rust
// this call will finish the initialization process
let server = service.serve(transport).await?;
```

Interact with the server

Once the server is initialized, you can send requests or notifications:

```rust
// request
let roots = server.list_roots().await?;

// or send notification
server.notify_cancelled(...).await?;
```

Waiting for service shutdown

```rust
let quit_reason = server.waiting().await?;
// or cancel it
let quit_reason = server.cancel().await?;
```

## Examples

See [examples](https://github.com/modelcontextprotocol/rust-sdk/blob/main/examples/README.md)

## OAuth Support

See [oauth_support](https://github.com/modelcontextprotocol/rust-sdk/blob/main/docs/OAUTH_SUPPORT.md) for details.

## Related Resources

- [MCP Specification](https://spec.modelcontextprotocol.io/specification/2024-11-05/)
- [Schema](https://github.com/modelcontextprotocol/specification/blob/main/schema/2024-11-05/schema.ts)

## Related Projects

### Extending `rmcp`

- [rmcp-actix-web](https://gitlab.com/lx-industries/rmcp-actix-web) - An `actix_web` backend for `rmcp`
- [rmcp-openapi](https://gitlab.com/lx-industries/rmcp-openapi) - Transform OpenAPI definition endpoints into MCP tools

### Built with `rmcp`

- [rustfs-mcp](https://github.com/rustfs/rustfs/tree/main/crates/mcp) - High-performance MCP server providing S3-compatible object storage operations for AI/LLM integration
- [containerd-mcp-server](https://github.com/jokemanfire/mcp-containerd) - A containerd-based MCP server implementation
- [rmcp-openapi-server](https://gitlab.com/lx-industries/rmcp-openapi/-/tree/main/crates/rmcp-openapi-server) - High-performance MCP server that exposes OpenAPI definition endpoints as MCP tools
- [nvim-mcp](https://github.com/linw1995/nvim-mcp) - A MCP server to interact with Neovim

## Development

### Tips for Contributors

See [docs/CONTRIBUTE.MD](https://github.com/modelcontextprotocol/rust-sdk/blob/main/docs/CONTRIBUTE.MD) to get some tips for contributing.

### Using Dev Container

If you want to use dev container, see [docs/DEVCONTAINER.md](https://github.com/modelcontextprotocol/rust-sdk/blob/main/docs/DEVCONTAINER.md) for instructions on using Dev Container for development.
