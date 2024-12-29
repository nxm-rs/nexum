use alloy_signer_local::PrivateKeySigner;
use clap::Parser;
use tui::app::App;

#[derive(Parser)]
struct Args {
    keystore_file: String,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let mut terminal = ratatui::init();
    terminal.clear()?;
    let app_result = App::default().run(terminal, args.keystore_file);
    ratatui::restore();
    app_result
}
