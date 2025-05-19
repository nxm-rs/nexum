#![feature(let_chains)]

use std::{
    fs::OpenOptions,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    sync::{RwLock, RwLockWriteGuard},
    time::Duration,
};

use ::rpc::run_server;
use alloy::signers::{k256::ecdsa::SigningKey, local::LocalSigner};
use clap::Parser;
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent};
use eyre::OptionExt;
use futures::StreamExt;
use ratatui::{
    layout::{Constraint, Layout},
    prelude::{Buffer, Rect},
    style::{Color, Style, Stylize},
    text::Text,
    widgets::{Block, Borders, FrameExt, List, ListState, Paragraph, StatefulWidget, Widget},
    DefaultTerminal, Frame,
};
use tokio::sync::mpsc;
use tracing_subscriber::EnvFilter;

/// Returns the base config directory for nexum. It also creates the directory
/// if it doesn't exist yet.
fn config_dir() -> eyre::Result<PathBuf> {
    let dir = std::env::home_dir()
        .ok_or_eyre("home directory not found")?
        .join(".nxm");
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

#[derive(Parser)]
struct Args {
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    host: IpAddr,
    #[arg(short, long, default_value = "1248")]
    port: u16,
    #[arg(short, long, default_value = "wss://eth.drpc.org")]
    rpc_url: String,
}

impl Args {
    fn listen_addr(&self) -> SocketAddr {
        SocketAddr::new(self.host, self.port)
    }
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(tui_logger)
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    let rpc_handle = run_server(args.listen_addr(), &args.rpc_url).await?;

    let terminal = ratatui::init();

    // run the loop until the tui quits or the server quits
    let app_result = tokio::select! {
        app_result = App::default().run(terminal) => { app_result }
        _ = rpc_handle.stopped() => { Ok(()) }
    };
    ratatui::restore();
    app_result
}

enum AppPane {
    Wallet,
    Dashboard,
}

impl Default for AppPane {
    fn default() -> Self {
        Self::Wallet
    }
}

impl AppPane {
    fn next(&self) -> Self {
        match self {
            Self::Wallet => Self::Dashboard,
            Self::Dashboard => Self::Wallet,
        }
    }
}

struct App {
    pub should_quit: bool,
    pub active_pane: AppPane,
    wallet_pane: WalletPane,
    prompt: Option<String>,
    prompt_input: String,
    prompt_receiver: mpsc::UnboundedReceiver<String>,
}

impl Default for App {
    fn default() -> Self {
        let mut list_state = ListState::default();
        list_state.select_first();

        // this is unbounded because unbounded's sender.send is sync
        let (sender, receiver) = mpsc::unbounded_channel();
        Self {
            should_quit: false,
            active_pane: AppPane::Wallet,
            wallet_pane: WalletPane {
                is_active: true,
                keystores: load_keystores().unwrap_or_default(),
                list_state: RwLock::new(list_state),
                active_wallet_idx: None,
                prompt_sender: sender.clone(),
            },
            prompt: None,
            prompt_input: "".to_string(),
            prompt_receiver: receiver,
        }
    }
}

impl App {
    const FRAMES_PER_SECOND: u64 = 60;

    pub async fn run(mut self, mut terminal: DefaultTerminal) -> eyre::Result<()> {
        let period = Duration::from_secs_f32(1.0 / (Self::FRAMES_PER_SECOND as f32));
        let mut interval = tokio::time::interval(period);
        let mut events = EventStream::new();

        while !self.should_quit {
            tokio::select! {
                _ = interval.tick() => { terminal.draw(|f| self.render(f).expect("failed to render"))?; },
                Some(Ok(event)) = events.next() => self.handle_event(&event),
                Some(prompt) = self.prompt_receiver.recv() => self.prompt = Some(prompt),
            }
        }
        Ok(())
    }

    fn render(&mut self, frame: &mut Frame) -> eyre::Result<()> {
        let horizontal = Layout::horizontal([Constraint::Ratio(1, 5), Constraint::Ratio(4, 5)]);
        let [left_area, right_area] = horizontal.areas(frame.area());

        let active_border_style = Style::default().fg(Color::Blue);
        let inactive_border_style = Style::default();

        frame.render_widget_ref(&self.wallet_pane, left_area);

        let dashboard_block = Block::default()
            .title("Dashboard")
            .borders(Borders::ALL)
            .border_style(match self.active_pane {
                AppPane::Wallet => inactive_border_style,
                AppPane::Dashboard => active_border_style,
            });
        frame.render_widget(dashboard_block, right_area);

        // render the popup password prompt
        self.render_prompt(frame);

        Ok(())
    }

    fn render_prompt(&mut self, frame: &mut Frame) {
        if let Some(prompt) = &self.prompt {
            let masked_pwd = "*".repeat(self.prompt_input.len());
            let paragraph = Paragraph::new(masked_pwd).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" {prompt} "))
                    .border_style(Style::default().fg(Color::Blue)),
            );
            let prompt_area = frame.area().centered(
                Constraint::Length(
                    prompt
                        .len()
                        .max(52)
                        .try_into()
                        .expect("cannot convert to u16"),
                ),
                Constraint::Length(3),
            );
            frame.render_widget(paragraph, prompt_area);
        }
    }

    fn handle_event(&mut self, event: &Event) {
        if let Some(key) = event.as_key_press_event() {
            match self.prompt {
                Some(_) => match key.code {
                    KeyCode::Char(ch) => self.prompt_input.push(ch),
                    KeyCode::Backspace => {
                        self.prompt_input.pop();
                    }
                    KeyCode::Esc => {
                        self.prompt = None;
                        self.prompt_input.clear();
                    }
                    KeyCode::Enter => {
                        self.prompt = None;
                        let input = self.prompt_input.clone();
                        self.prompt_input.clear();
                        self.wallet_pane.on_prompt_input(input);
                    }
                    _ => {}
                },
                None => match (&self.active_pane, key.code) {
                    (_, KeyCode::Char('q') | KeyCode::Esc) => self.should_quit = true,
                    (_, KeyCode::Tab) => {
                        self.active_pane = self.active_pane.next();
                        self.wallet_pane
                            .set_is_active(matches!(self.active_pane, AppPane::Wallet));
                    }
                    (AppPane::Wallet, _) => self.wallet_pane.handle_key(&key),
                    _ => {}
                },
            }
        }
    }
}

/// Returns all the keystore file paths in the foundry keystore directory.
fn load_keystores() -> eyre::Result<Vec<KeystoreWallet>> {
    let home_dir = std::env::home_dir().ok_or_eyre("home directory not found")?;
    let foundry_keystore_dir = home_dir.join(".foundry/keystores");
    let foundry_keystore_files = std::fs::read_dir(foundry_keystore_dir)?;

    Ok(foundry_keystore_files
        .into_iter()
        .filter_map(|f| {
            f.ok().map(|f| KeystoreWallet {
                path: f.path(),
                name: f.file_name().to_string_lossy().to_string(),
                signer: None,
            })
        })
        .collect::<Vec<_>>())
}

trait HandleEvent {
    fn handle_key(&mut self, event: &KeyEvent);
}

struct KeystoreWallet {
    name: String,
    path: PathBuf,
    signer: Option<LocalSigner<SigningKey>>,
}

impl KeystoreWallet {
    /// Returns if the keystore is locked.
    fn is_locked(&self) -> bool {
        self.signer.is_none()
    }

    /// Try to decrypt the keystore with given password.
    fn try_unlock(&mut self, password: String) -> eyre::Result<()> {
        let signer = LocalSigner::<SigningKey>::decrypt_keystore(&self.path, password)?;
        self.signer = Some(signer);
        Ok(())
    }
}

struct WalletPane {
    is_active: bool,
    keystores: Vec<KeystoreWallet>,
    list_state: RwLock<ListState>,
    active_wallet_idx: Option<usize>,
    prompt_sender: mpsc::UnboundedSender<String>,
}

impl Widget for &WalletPane {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut list_state = self
            .list_state
            .write()
            .expect("failed to get write lock on list state");
        let list = List::new(
            self.keystores
                .iter()
                .enumerate()
                .map(|(idx, k)| {
                    let name = Text::from(format!(
                        "{} {}",
                        if k.is_locked() { "🔒" } else { "🔓" },
                        k.name.clone()
                    ));
                    if let Some(active_wallet_idx) = self.active_wallet_idx
                        && idx == active_wallet_idx
                    {
                        name.style(Style::default().bold().fg(Color::Blue))
                    } else {
                        name
                    }
                })
                .collect::<Vec<_>>(),
        )
        .highlight_symbol("> ")
        .highlight_style(Style::default().reversed())
        .block(
            Block::default()
                .title("Wallets")
                .borders(Borders::ALL)
                .border_style(if self.is_active {
                    Style::default().fg(Color::Blue)
                } else {
                    Style::default()
                }),
        );
        StatefulWidget::render(list, area, buf, &mut *list_state);
    }
}

impl WalletPane {
    fn select_next(&self) {
        let list_state = &mut *self.get_list_state_w();
        if let Some(selected_idx) = list_state.selected() {
            if selected_idx < self.keystores.len() - 1 {
                list_state.select_next();
            } else {
                list_state.select_first();
            }
        } else {
            list_state.select_first();
        }
    }

    fn select_previous(&self) {
        let list_state = &mut *self.get_list_state_w();
        if let Some(selected_idx) = list_state.selected() {
            if selected_idx > 0 {
                list_state.select_previous();
            } else {
                list_state.select_last();
            }
        } else {
            list_state.select_last();
        }
    }

    fn get_list_state_w(&self) -> RwLockWriteGuard<'_, ListState> {
        self.list_state
            .write()
            .expect("failed to get write lock on list state")
    }

    fn set_is_active(&mut self, is_active: bool) {
        self.is_active = is_active;
    }

    fn set_active_wallet_to_selected_index(&mut self) -> Option<usize> {
        let list_state = self
            .list_state
            .read()
            .expect("failed to get read lock on list state");
        let idx = list_state.selected();
        self.active_wallet_idx = idx;
        idx
    }

    fn on_prompt_input(&mut self, input: String) {
        if let Some(idx) = self.active_wallet_idx
            && self.keystores[idx].is_locked()
        {
            let keystore = &mut self.keystores[idx];
            if keystore.try_unlock(input).is_err() {
                self.prompt_sender
                    .send(format!(
                        "[{}] Incorrect password! Try again.",
                        keystore.name
                    ))
                    .expect("sending password retry prompt failed");
            }
        }
    }
}

impl HandleEvent for WalletPane {
    fn handle_key(&mut self, key: &KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.select_previous(),
            KeyCode::Down | KeyCode::Char('j') => self.select_next(),
            KeyCode::Enter => {
                if let Some(idx) = self.set_active_wallet_to_selected_index()
                    && self.keystores[idx].is_locked()
                {
                    let keystore = &self.keystores[idx];
                    self.prompt_sender
                        .send(format!("Enter password for {}", keystore.name))
                        .expect("sending password prompt request failed");
                }
            }
            _ => {}
        }
    }
}
