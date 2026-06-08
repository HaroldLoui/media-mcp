use anyhow::Result;
use clap::Parser;
use media_mcp::config::Config;
use media_mcp::handlers::MediaServer;
use rmcp::{ServiceExt, transport::stdio};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "media-mcp", about = "MCP server for multimedia file reading")]
struct Args {
    /// Path to config file
    #[arg(long)]
    config: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let args = Args::parse();
    let config = Config::load(args.config.as_deref())?;

    // Set TESSDATA_PREFIX for tesseract if configured
    if let Some(ref cmd) = config.ocr.engines.tesseract.tesseract_cmd {
        // SAFETY: Called once at startup before any threads read the env
        unsafe { std::env::set_var("TESSDATA_PREFIX", cmd) };
    }

    tracing::info!(
        "media-mcp starting: model={}, ocr_langs={}",
        config.vision_api.model,
        config.languages_string()
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(config.vision_api.timeout_seconds))
        .build()?;

    let server = MediaServer { config, client };
    let service = server.serve(stdio()).await.inspect_err(|e| {
        tracing::error!("MCP serve error: {:?}", e);
    })?;

    service.waiting().await?;
    Ok(())
}
