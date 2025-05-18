use std::{fs::OpenOptions, path::PathBuf};

use eyre::OptionExt;
use tracing_subscriber::EnvFilter;

/// Returns the base config directory for nexum. It also creates the directory
/// if it doesn't exist yet.
fn config_dir() -> eyre::Result<PathBuf> {
    let dir = std::env::home_dir()
        .ok_or_eyre("home directory not found")?
        .join(".nexum");
    if !dir.exists() {
        std::fs::create_dir(&dir)?
    }
    Ok(dir)
}

fn tui_logger() -> impl std::io::Write {
    let log_file = config_dir()
        .expect("failed to get config dir")
        .join("nxm.log");
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)
        .expect("failed to open log file")
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_writer(tui_logger)
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    tracing::info!("info");
    tracing::warn!("warn");
    tracing::error!("error");
    tracing::debug!("debug");
    tracing::trace!("trace");
}
