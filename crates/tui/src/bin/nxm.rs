use clap::Parser;
use tui::{
    app::App,
    widgets::prompts::{ConnectionPrompt, PendingPromptsState, Prompt, TransactionRequestPrompt},
};

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

    let app = App {
        pending_prompts_state: PendingPromptsState {
            prompts: vec![
                Prompt::Connection(ConnectionPrompt {
                    origin: "https://swap.cow.fi".into(),
                }),
                Prompt::Connection(ConnectionPrompt {
                    origin: "https://app.safe.global".into(),
                }),
                Prompt::TransactionRequest(TransactionRequestPrompt {
                    to: Default::default(),
                    gas_limit: 0,
                    gas_price: 0,
                    value: Default::default(),
                    data: Default::default(),
                }),
            ],
            prompts_list_state: Default::default(),
        },
        ..Default::default()
    };
    let app_result = app.run(terminal, args.keystore_file);
    ratatui::restore();
    app_result
}
