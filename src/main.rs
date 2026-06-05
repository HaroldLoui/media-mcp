mod config;

use anyhow::Result;
use clap::Parser;
use config::Config;
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
    tracing::info!(
        "Config loaded: model={}, ocr_langs={}",
        config.vision_api.model,
        config.languages_string()
    );

    Ok(())
}
