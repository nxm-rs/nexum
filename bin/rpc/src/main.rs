#![cfg(not(target_arch = "wasm32"))]

use clap::Parser;
use cli::Cli;

mod cli;
mod config;
mod logging;
mod namespaces;
mod rpc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    crate::logging::init()?;

    let args = Cli::parse();
    let handle = rpc::run(args.listen_addr, args.rpc_url).await?;
    let handler = tokio::spawn(handle.stopped());

    handler.await?;

    // // Run the TUI
    Ok(())
}
