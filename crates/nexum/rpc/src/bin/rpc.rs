use std::net::Ipv4Addr;

use alloy_chains::NamedChain;
use clap::Parser;
use eyre::OptionExt;
use nexum_rpc::rpc::{chain_id_or_name_to_named_chain, RpcServerBuilder};
use url::Url;

#[derive(Parser, Debug)]
#[command(about)]
pub struct Args {
    #[arg(short, long, default_value = "127.0.0.1")]
    pub host: Ipv4Addr,

    #[arg(short, long, default_value = "1248")]
    pub port: u16,

    /// Node JSON-RPC URLs in the format `<chain>=<url>`
    #[arg(short, long)]
    pub rpc_urls: Vec<String>,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let args = Args::parse();
    tracing_subscriber::fmt::init();

    let mut builder = RpcServerBuilder::new().host(args.host).port(args.port);

    for (chain, url) in args
        .rpc_urls
        .iter()
        .map(|s: &String| -> eyre::Result<(NamedChain, Url)> {
            let (chain, rpc) = s.split_once("=").ok_or_eyre("invalid format for rpc url")?;
            let chain = chain_id_or_name_to_named_chain(chain)?;
            Ok((chain, rpc.parse()?))
        })
        .filter_map(Result::ok)
    {
        builder = builder.chain(chain, url);
    }
    let mut rpc = builder.build().await;
    let (handle, _) = rpc.run().await?;
    handle.stopped().await;

    Ok(())
}
