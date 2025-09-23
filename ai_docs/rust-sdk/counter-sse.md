# Counter SSE Server Example

This is a Rust example demonstrating how to create an MCP (Model Context Protocol) server using Server-Sent Events (SSE) transport.

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

1. **SSE Server Setup**: Creates an SSE server configuration with specific bind address and paths
2. **Tracing Integration**: Sets up structured logging with the `tracing` crate
3. **Graceful Shutdown**: Implements proper cancellation token handling for clean shutdown
4. **Axum Integration**: Uses the Axum web framework to serve the SSE endpoint
5. **Counter Service**: Integrates a counter service that implements MCP functionality

## Key Components

- **SseServerConfig**: Configuration for the SSE server including bind address, SSE path (`/sse`), and POST path (`/message`)
- **Cancellation Tokens**: Used for coordinating graceful shutdown across async tasks
- **Counter Service**: A service that provides MCP functionality over the SSE transport

## Usage

The server binds to `127.0.0.1:8000` and provides:
- SSE endpoint at `/sse` for receiving server-sent events
- POST endpoint at `/message` for sending messages to the server

The server runs until receiving a Ctrl+C signal, at which point it gracefully shuts down.