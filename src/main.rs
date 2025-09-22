use clap::Parser;
use rmcp::transport::sse_server::{SseServer, SseServerConfig};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod tools;
use tools::GeminiCli;

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    /// Path or command to gemini-cli executable (supports multi-word commands like "task ai:run")
    #[arg(long, env = "GEMINI_CLI_COMMAND", default_value = "gemini-cli")]
    gemini_cli_command: String,

    /// Hostname to bind the server to
    #[arg(long, env = "MCP_GEMINI_CLI_HOSTNAME", default_value = "127.0.0.1")]
    hostname: String,

    /// Port to bind the server to
    #[arg(long, env = "MCP_GEMINI_CLI_PORT", default_value = "8000")]
    port: u16,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let bind_address = format!("{}:{}", args.hostname, args.port);
    tracing::info!("Starting MCP SSE Server on {}", bind_address);

    // Configure SSE server
    let config = SseServerConfig {
        bind: bind_address.parse()?,
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

    // Start the MCP service with GeminiCli tools
    let gemini_cli_command = args.gemini_cli_command.clone();
    let ct = sse_server.with_service(move || GeminiCli::new(gemini_cli_command.clone()));

    tracing::info!("MCP SSE Server running!");
    tracing::info!("SSE endpoint: http://{}/sse", bind_address);
    tracing::info!("Message endpoint: http://{}/message", bind_address);
    tracing::info!("Test with MCP Inspector: https://github.com/modelcontextprotocol/inspector");
    tracing::info!("Press Ctrl+C to stop");

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutdown signal received");
    ct.cancel();

    Ok(())
}
