use rmcp::transport::sse_server::{SseServer, SseServerConfig};
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

mod tools;
use tools::FileSystem;

const BIND_ADDRESS: &str = "127.0.0.1:8000";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting MCP SSE Server on {}", BIND_ADDRESS);

    // Configure SSE server
    let config = SseServerConfig {
        bind: BIND_ADDRESS.parse()?,
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        ct: tokio_util::sync::CancellationToken::new(),
        sse_keep_alive: None,
    };

    let (sse_server, router) = SseServer::new(config);

    // Start the HTTP server
    let listener = tokio::net::TcpListener::bind(sse_server.config.bind).await?;
    let ct = sse_server.config.ct.child_token();

    let server = axum::serve(listener, router).with_graceful_shutdown(async move {
        ct.cancelled().await;
        tracing::info!("SSE server gracefully shutting down");
    });

    tokio::spawn(async move {
        if let Err(e) = server.await {
            tracing::error!(error = %e, "SSE server shutdown with error");
        }
    });

    // Start the MCP service with FileSystem tools
    let ct = sse_server.with_service(FileSystem::new);

    tracing::info!("MCP SSE Server running!");
    tracing::info!("SSE endpoint: http://{}/sse", BIND_ADDRESS);
    tracing::info!("Message endpoint: http://{}/message", BIND_ADDRESS);
    tracing::info!("Test with MCP Inspector: https://github.com/modelcontextprotocol/inspector");
    tracing::info!("Press Ctrl+C to stop");

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutdown signal received");
    ct.cancel();

    Ok(())
}