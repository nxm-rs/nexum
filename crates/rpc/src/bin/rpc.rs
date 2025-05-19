use clap::Parser;

#[derive(Parser, Debug)]
#[command(about)]
pub struct Args {
    /// Address to listen for browser extension WebSocket requests
    #[arg(short, long, value_name = "ADDR", default_value = "127.0.0.1:1248")]
    pub listen_addr: String,

    /// Node JSON-RPC URL capable of supporting WebSockets
    #[arg(
        short,
        long,
        value_name = "RPC_URL",
        default_value = "wss://eth.drpc.org"
    )]
    pub rpc_url: String,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let args = Args::parse();
    tracing_subscriber::fmt::init();
    rpc::run_server(args.listen_addr.parse()?, args.rpc_url.as_str())
        .await?
        .stopped()
        .await;

    Ok(())
}
