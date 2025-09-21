# Counter SSE Server Example

This is an example of a Counter server using Server-Sent Events (SSE) transport from the MCP Rust SDK.

## Source Code

```rust
use rmcp::transport::sse_server::{SseServer, SseServerConfig};
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt,
    {self},
};

mod common;
use common::counter::Counter;

const BIND_ADDRESS: &str = "127.0.0.1:8000";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = SseServerConfig {
        bind: BIND_ADDRESS.parse()?,
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        ct: tokio_util::sync::CancellationToken::new(),
        sse_keep_alive: None,
    };

    let (sse_server, router) = SseServer::new(config);

    // Do something with the router, e.g., add routes or middleware
    let listener = tokio::net::TcpListener::bind(sse_server.config.bind).await?;
    let ct = sse_server.config.ct.child_token();

    let server = axum::serve(listener, router).with_graceful_shutdown(async move {
        ct.cancelled().await;
        tracing::info!("sse server cancelled");
    });

    tokio::spawn(async move {
        if let Err(e) = server.await {
            tracing::error!(error = %e, "sse server shutdown with error");
        }
    });

    let ct = sse_server.with_service(Counter::new);

    tokio::signal::ctrl_c().await?;
    ct.cancel();

    Ok(())
}
```

## Description

This example demonstrates:

1. **SSE Server Setup**: Creates an SSE server with configuration for bind address, SSE endpoint path, and message endpoint path
2. **Tracing**: Sets up structured logging with tracing subscriber
3. **Graceful Shutdown**: Implements graceful shutdown handling with cancellation tokens
4. **Counter Service**: Integrates with a Counter MCP service
5. **Axum Integration**: Uses Axum web framework for the HTTP server

## Key Components

- **SseServer**: The main SSE server implementation
- **SseServerConfig**: Configuration for the SSE server including bind address and paths
- **Counter**: The MCP service being exposed via SSE transport
- **Graceful Shutdown**: Proper cleanup when the server is terminated

## Usage

This server binds to `127.0.0.1:8000` and provides:
- SSE endpoint at `/sse` for real-time communication
- Message endpoint at `/message` for sending messages to the server

The server runs until Ctrl+C is pressed, at which point it gracefully shuts down.