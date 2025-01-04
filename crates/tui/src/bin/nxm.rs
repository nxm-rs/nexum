use clap::Parser;
use tui::app::App;

#[derive(Parser)]
struct Args {
    keystore_file: String,
    #[clap(flatten)]
    rpc: rpc::Cli,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let runtime = tokio::runtime::Builder::new_multi_thread().build()?;
    let server_handle = rpc::run(args.rpc.listen_addr.clone(), args.rpc.rpc_url.clone());
    let _ = std::thread::spawn(move || {
        let _ = runtime.block_on(server_handle);
    });

    let mut terminal = ratatui::init();
    terminal.clear()?;
    let app_result = App::default().run(terminal, args.keystore_file);
    ratatui::restore();
    app_result
}
