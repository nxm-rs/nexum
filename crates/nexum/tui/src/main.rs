#![feature(let_chains)]

use std::{
    fs::OpenOptions,
    net::Ipv4Addr,
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
    time::Duration,
};

use alloy::{
    consensus::{EthereumTypedTransaction, SignableTransaction, TxEip4844Variant},
    dyn_abi::TypedData,
    primitives::{Address, B256, Bytes},
    signers::Signature,
};
use alloy_chains::NamedChain;
use clap::Parser;
use config_tab::ConfigTab;
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent};
use futures::StreamExt;
use nexum_rpc::rpc::{
    InteractiveRequest, InteractiveResponse, RpcServerBuilder, chain_id_or_name_to_named_chain,
};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, HorizontalAlignment, Layout},
    prelude::{Buffer, Rect},
    style::{Color, Style, Stylize},
    symbols,
    text::Text,
    widgets::{
        Block, Borders, FrameExt, List, ListState, Padding, Paragraph, StatefulWidget, Tabs, Widget,
    },
};
use signers::{NexumAccount, load_ledger_accounts};
use tokio::sync::{mpsc, oneshot};
use tracing_subscriber::EnvFilter;

use config::{Config, config_dir, load_config};
use url::Url;

mod config;
mod config_tab;
mod signers;

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
    host: Ipv4Addr,
    #[arg(short, long, default_value = "1248")]
    port: u16,
    #[arg(short, long)]
    rpc_urls: Vec<String>,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(tui_logger)
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    let config = load_config()?;
    tracing::debug!(?config, formatted = ?toml::to_string_pretty(&config)?);

    let mut builder = RpcServerBuilder::new().host(args.host).port(args.port);
    let mut rpcs = config.chain_rpcs().await?;
    rpcs.extend(
        args.rpc_urls
            .iter()
            .map(|s: &String| -> eyre::Result<(NamedChain, Url)> {
                let (chain, rpc) = s
                    .split_once("=")
                    .ok_or_else(|| eyre::eyre!("invalid format for rpc url"))?;
                let chain = chain_id_or_name_to_named_chain(chain)?;
                Ok((chain, rpc.parse()?))
            })
            .collect::<eyre::Result<Vec<_>>>()?,
    );
    // since the cli rpcs are added after the config rpcs, the cli rpcs will override
    // the config rpcs if the same chain is specified
    for (chain, url) in rpcs {
        builder = builder.chain(chain, url);
    }

    let mut rpc = builder.build().await;
    let (srv_handle, req_receiver) = rpc.run().await?;

    let terminal = ratatui::init();

    let app = App::new(req_receiver, config).await;
    // run the loop until the tui quits or the server quits
    let app_result = tokio::select! {
        app_result = app.run(terminal) => { app_result }
        _ = srv_handle.stopped() => { Ok(()) }
    };
    ratatui::restore();
    app_result
}

enum AppPane {
    Tabs,
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
            Self::Tabs => Self::Wallet,
            Self::Wallet => Self::Dashboard,
            Self::Dashboard => Self::Tabs,
        }
    }
}

enum AppTab {
    Main,
    Settings,
}

impl Default for AppTab {
    fn default() -> Self {
        Self::Main
    }
}

impl AppTab {
    fn next(&self) -> Self {
        match self {
            Self::Main => Self::Settings,
            Self::Settings => Self::Main,
        }
    }

    fn prev(&self) -> Self {
        match self {
            Self::Main => Self::Settings,
            Self::Settings => Self::Main,
        }
    }

    fn id_to_tab(id: usize) -> Option<Self> {
        match id {
            1 => Some(Self::Main),
            2 => Some(Self::Settings),
            _ => None,
        }
    }

    fn to_id(&self) -> usize {
        match self {
            Self::Main => 0,
            Self::Settings => 1,
        }
    }
}

struct App {
    pub should_quit: bool,
    pub active_app_pane: AppPane,
    active_tab: AppTab,
    wallet_pane: Arc<WalletPane>,
    prompt: Option<Prompt>,
    prompt_input: String,
    prompt_receiver: mpsc::UnboundedReceiver<Prompt>,
    prompt_sender: mpsc::UnboundedSender<Prompt>,
    request_receiver: mpsc::Receiver<(InteractiveRequest, oneshot::Sender<InteractiveResponse>)>,
    config_tab: ConfigTab,
}

impl App {
    const FRAMES_PER_SECOND: u64 = 60;

    async fn new(
        request_receiver: mpsc::Receiver<(
            InteractiveRequest,
            oneshot::Sender<InteractiveResponse>,
        )>,
        config: Config,
    ) -> Self {
        let mut list_state = ListState::default();
        list_state.select_first();

        // this is unbounded because unbounded's sender.send is sync
        let (sender, receiver) = mpsc::unbounded_channel();

        Self {
            should_quit: false,
            active_app_pane: AppPane::Wallet,
            active_tab: AppTab::default(),
            wallet_pane: Arc::new(WalletPane {
                is_active: RwLock::new(true),
                accounts: RwLock::new(
                    config
                        .keystores()
                        .inspect_err(|err| tracing::error!(?err, "error loading foundry keystores"))
                        .unwrap_or_default(),
                ),
                list_state: RwLock::new(list_state),
                active_wallet_idx: RwLock::new(None),
                prompt_sender: sender.clone(),
            }),
            prompt: None,
            prompt_input: "".to_string(),
            prompt_sender: sender.clone(),
            prompt_receiver: receiver,
            request_receiver,
            config_tab: ConfigTab::new(config),
        }
    }

    pub async fn run(mut self, mut terminal: DefaultTerminal) -> eyre::Result<()> {
        let period = Duration::from_secs_f32(1.0 / (Self::FRAMES_PER_SECOND as f32));
        let mut interval = tokio::time::interval(period);
        let mut events = EventStream::new();

        // load ledger accounts in background because its too slow
        let wallet_pane_clone = self.wallet_pane.clone();
        tokio::spawn(async move {
            load_ledger_accounts(self.config_tab.config.signer.ledger.n)
                .await
                .map(|accounts| wallet_pane_clone.add_accounts(accounts))
                .ok();
        });

        while !self.should_quit {
            tokio::select! {
                _ = interval.tick() => { terminal.draw(|f| self.render(f).expect("failed to render"))?; },
                Some(Ok(event)) = events.next() => self.handle_event(&event),
                Some(prompt) = self.prompt_receiver.recv(), if self.prompt.is_none() => self.prompt = Some(prompt),
                Some((req, res_sender)) = self.request_receiver.recv() => {
                    // TODO: this probably shouldn't be awaited, will probably block the UI
                    self.handle_request(req, res_sender).await;
                },
            }
        }
        Ok(())
    }

    fn render(&mut self, frame: &mut Frame) -> eyre::Result<()> {
        let full_area = frame.area();
        let active_border_style = Style::default().fg(Color::Blue);
        let inactive_border_style = Style::default();

        // render the tabs
        let tabs = Tabs::new(
            vec!["Wallet", "Settings"]
                .into_iter()
                .enumerate()
                .map(|(idx, s)| format!("{s}[{}]", idx + 1))
                .collect::<Vec<_>>(),
        )
        .block(
            Block::bordered().border_style(if matches!(self.active_app_pane, AppPane::Tabs) {
                active_border_style
            } else {
                inactive_border_style
            }),
        )
        .highlight_style(Style::default().bg(Color::Blue).fg(Color::Black).bold())
        .select(self.active_tab.to_id())
        .divider(symbols::line::VERTICAL)
        .padding(" ", " ");
        let tab_area = Rect {
            x: 0,
            y: 0,
            width: full_area.width,
            height: 3,
        };
        frame.render_widget(tabs, tab_area);

        let tab_inner = Rect {
            x: full_area.x,
            y: full_area.y + 3,
            width: full_area.width,
            height: full_area.height - 3,
        };
        match self.active_tab {
            AppTab::Main => {
                let horizontal =
                    Layout::horizontal([Constraint::Ratio(1, 5), Constraint::Ratio(4, 5)]);
                let [left_area, right_area] = horizontal.areas(tab_inner);

                frame.render_widget_ref(&*self.wallet_pane, left_area);
                let dashboard_block = Block::default()
                    .title({
                        let active_account = self.wallet_pane.active_account();
                        let hovered_account = self.wallet_pane.hovered_account();
                        match (active_account, hovered_account) {
                            (Some(active), Some(hovered)) => {
                                format!(" Active: {active} ───── Hovered: {hovered} ",)
                            }
                            (Some(active), None) => format!(" Active: {active} "),
                            (None, Some(hovered)) => format!(" Hovered: {hovered} "),
                            _ => " Dashboard ".to_string(),
                        }
                    })
                    .borders(Borders::ALL)
                    .border_style(match self.active_app_pane {
                        AppPane::Wallet => inactive_border_style,
                        AppPane::Dashboard => active_border_style,
                        AppPane::Tabs => inactive_border_style,
                    });
                frame.render_widget(dashboard_block, right_area);

                // render the popup password prompt
                self.render_prompt(frame);
            }
            AppTab::Settings => {
                frame.render_widget_ref(&self.config_tab, tab_inner);
            }
        }

        Ok(())
    }

    fn render_prompt(&mut self, frame: &mut Frame) {
        if let Some(prompt) = &self.prompt {
            match prompt {
                Prompt::AccountUnlock(name) => {
                    let masked_pwd = "*".repeat(self.prompt_input.len());
                    let prompt_str = format!(" Enter password for {name} ");
                    let prompt_area = frame.area().centered(
                        Constraint::Length(
                            prompt_str
                                .len()
                                .max(52)
                                .try_into()
                                .expect("cannot convert to u16"),
                        ),
                        Constraint::Length(3),
                    );
                    let paragraph = Paragraph::new(masked_pwd).block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(prompt_str)
                            .border_style(Style::default().fg(Color::Blue)),
                    );
                    frame.render_widget(paragraph, prompt_area);
                }
                Prompt::AccountUnlockInvalidPasswordRetry(name) => {
                    let masked_pwd = "*".repeat(self.prompt_input.len());
                    let prompt_str = format!(" Incorrect password for {name}! Try again. ");
                    let prompt_area = frame.area().centered(
                        Constraint::Length(
                            prompt_str
                                .len()
                                .max(52)
                                .try_into()
                                .expect("cannot convert to u16"),
                        ),
                        Constraint::Length(3),
                    );
                    let paragraph = Paragraph::new(masked_pwd).block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(prompt_str)
                            .border_style(Style::default().fg(Color::Blue)),
                    );
                    frame.render_widget(paragraph, prompt_area);
                }
                Prompt::SendTransaction(req, _) => {
                    let block = Block::bordered()
                        .padding(Padding::uniform(1))
                        .title(" Send Transaction ")
                        .title_alignment(HorizontalAlignment::Center)
                        .title_bottom("[A]ccept ───── [R]eject");
                    let text = match req.as_ref() {
                        EthereumTypedTransaction::Legacy(tx_legacy) => {
                            format!("{tx_legacy:#?}")
                        }
                        EthereumTypedTransaction::Eip2930(tx_eip2930) => format!("{tx_eip2930:#?}"),
                        EthereumTypedTransaction::Eip1559(tx_eip1559) => format!("{tx_eip1559:#?}"),
                        EthereumTypedTransaction::Eip4844(tx_eip4844) => format!("{tx_eip4844:#?}"),
                        EthereumTypedTransaction::Eip7702(tx_eip7702) => format!("{tx_eip7702:#?}"),
                    };

                    let n_lines = text.lines().count();
                    let para = Paragraph::new(text).block(block);
                    let prompt_area = frame.area().centered(
                        Constraint::Length(80),
                        Constraint::Length(n_lines as u16 + 4),
                    );
                    frame.render_widget(para, prompt_area);
                }
                Prompt::EthSign(_, message, _) => {
                    let block = Block::bordered()
                        .padding(Padding::uniform(1))
                        .title(" Sign EIP-191 Message ")
                        .title_alignment(HorizontalAlignment::Center)
                        .title_bottom("[A]ccept ───── [R]eject");
                    frame.render_widget(
                        Paragraph::new(message.to_string()).block(block),
                        frame
                            .area()
                            .centered(Constraint::Length(80), Constraint::Length(10 + 4)),
                    );
                }
                Prompt::EthSignTypedData(_, data, _) => {
                    let block = Block::bordered()
                        .padding(Padding::uniform(1))
                        .title(" Sign Typed Data ")
                        .title_alignment(HorizontalAlignment::Center)
                        .title_bottom("[A]ccept ───── [R]eject");
                    let text = format!("{data:#?}");
                    let n_lines = text.lines().count();
                    frame.render_widget(
                        Paragraph::new(text).block(block),
                        frame.area().centered(
                            Constraint::Length(80),
                            Constraint::Length((n_lines as u16) + 4),
                        ),
                    );
                }
            }
        }
    }

    fn handle_event(&mut self, event: &Event) {
        if let Some(key) = event.as_key_press_event() {
            match &self.prompt {
                Some(prompt) => match prompt {
                    Prompt::AccountUnlock(_) | Prompt::AccountUnlockInvalidPasswordRetry(_) => {
                        match key.code {
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
                        }
                    }
                    Prompt::SendTransaction(_, _) => match key.code {
                        KeyCode::Esc | KeyCode::Char('r') | KeyCode::Char('R') => {
                            if let Some(Prompt::SendTransaction(tx, sender)) = self.prompt.take() {
                                sender
                                    .send((tx, false))
                                    .expect("failed to send send transaction prompt response");
                            }
                        }
                        KeyCode::Char('a') | KeyCode::Char('A') => {
                            if let Some(Prompt::SendTransaction(tx, sender)) = self.prompt.take() {
                                sender
                                    .send((tx, true))
                                    .expect("failed to send send transaction prompt response");
                            }
                        }
                        _ => {}
                    },
                    Prompt::EthSign(_, _, _) => match key.code {
                        KeyCode::Esc | KeyCode::Char('r') | KeyCode::Char('R') => {
                            if let Some(Prompt::EthSign(signer_addr, message, sender)) =
                                self.prompt.take()
                            {
                                sender
                                    .send((signer_addr, message, false))
                                    .expect("failed to send eth_sign prompt response");
                            }
                        }
                        KeyCode::Char('a') | KeyCode::Char('A') => {
                            if let Some(Prompt::EthSign(signer_addr, message, sender)) =
                                self.prompt.take()
                            {
                                sender
                                    .send((signer_addr, message, true))
                                    .expect("failed to send eth_sign prompt response");
                            }
                        }
                        _ => {}
                    },
                    Prompt::EthSignTypedData(_, _, _) => match key.code {
                        KeyCode::Esc | KeyCode::Char('r') | KeyCode::Char('R') => {
                            if let Some(Prompt::EthSignTypedData(signer_addr, data, sender)) =
                                self.prompt.take()
                            {
                                sender
                                    .send((signer_addr, data, false))
                                    .expect("failed to send eth_sign_typed_data prompt response");
                            }
                        }
                        KeyCode::Char('a') | KeyCode::Char('A') => {
                            if let Some(Prompt::EthSignTypedData(signer_addr, data, sender)) =
                                self.prompt.take()
                            {
                                sender
                                    .send((signer_addr, data, true))
                                    .expect("failed to send eth_sign_typed_data prompt response");
                            }
                        }
                        _ => {}
                    },
                },
                None => match (&self.active_tab, key.code) {
                    // global keybinds
                    (_, KeyCode::Char('q') | KeyCode::Esc) => self.should_quit = true,
                    (_, KeyCode::Char('1')) => self.active_tab = AppTab::id_to_tab(1).unwrap(),
                    (_, KeyCode::Char('2')) => self.active_tab = AppTab::id_to_tab(2).unwrap(),
                    // main tab keybinds
                    (AppTab::Main, _) => match (&self.active_app_pane, key.code) {
                        (_, KeyCode::Tab) => {
                            self.active_app_pane = self.active_app_pane.next();
                            self.wallet_pane
                                .set_is_active(matches!(self.active_app_pane, AppPane::Wallet));
                        }
                        (AppPane::Wallet, _) => self.wallet_pane.handle_key(&key),
                        (AppPane::Tabs, KeyCode::Right | KeyCode::Char('l')) => {
                            self.active_tab = self.active_tab.next();
                        }
                        (AppPane::Tabs, KeyCode::Left | KeyCode::Char('h')) => {
                            self.active_tab = self.active_tab.prev();
                        }
                        _ => {}
                    },

                    // settings tab keybinds
                    (AppTab::Settings, _) => {
                        self.config_tab.handle_key(&key);
                    }
                },
            }
        }
    }

    async fn handle_request(
        &self,
        request: InteractiveRequest,
        response_sender: oneshot::Sender<InteractiveResponse>,
    ) {
        match request {
            InteractiveRequest::EthRequestAccounts => {
                response_sender
                    .send(InteractiveResponse::EthRequestAccounts(
                        if let Some(addr) = self.wallet_pane.active_account() {
                            vec![addr]
                        } else {
                            vec![]
                        },
                    ))
                    .inspect_err(|_| tracing::error!("failed to send eth_requestAccounts response"))
                    .ok();
            }
            InteractiveRequest::EthAccounts => {
                response_sender
                    .send(InteractiveResponse::EthAccounts(
                        if let Some(addr) = self.wallet_pane.active_account() {
                            vec![addr]
                        } else {
                            vec![]
                        },
                    ))
                    .inspect_err(|_| tracing::error!("failed to send eth_accounts response"))
                    .ok();
            }
            InteractiveRequest::SignTransaction(tx_req) => {
                let (sender, receiver) =
                    oneshot::channel::<(Box<EthereumTypedTransaction<TxEip4844Variant>>, bool)>();
                self.prompt_sender
                    .send(Prompt::SendTransaction(tx_req, sender))
                    .expect("failed to send send transaction prompt");
                let wallet = self.wallet_pane.clone();
                tokio::spawn(async move {
                    let (tx, should_sign) = receiver
                        .await
                        .expect("failed to receive send transaction response");
                    if should_sign {
                        tracing::debug!("signing and sending transaction now");
                        response_sender
                            .send(InteractiveResponse::SignTransaction(
                                wallet
                                    .sign_hash(None, &tx.signature_hash())
                                    .await
                                    .map_err(|e| {
                                        tracing::error!(?e, "failed to sign tx");
                                        let boxed_error: Box<dyn std::error::Error + Send + Sync> =
                                            Box::new(e);
                                        boxed_error
                                    }),
                            ))
                            .expect("failed to send send transaction response");
                    } else {
                        tracing::debug!("sending transaction rejected");
                        response_sender
                            .send(InteractiveResponse::SignTransaction(Err(Box::new(
                                NexumTuiError::UserRejectedSigning,
                            ))))
                            .expect("failed to send send transaction response");
                    }
                });
            }
            InteractiveRequest::EthSign(signer, message) => {
                let (sender, receiver) = oneshot::channel::<(Address, Bytes, bool)>();
                self.prompt_sender
                    .send(Prompt::EthSign(signer, message, sender))
                    .expect("failed to send eth_sign prompt");
                let wallet = self.wallet_pane.clone();
                tokio::spawn(async move {
                    let (signer, message, should_sign) =
                        receiver.await.expect("failed to receive eth_sign response");
                    if should_sign {
                        tracing::debug!("signing and sending transaction now");
                        response_sender
                            .send(InteractiveResponse::EthSign(
                                wallet
                                    .sign_message(Some(signer), &message)
                                    .await
                                    .map_err(|e| {
                                        tracing::error!(?e, "failed to sign message");
                                        let boxed_error: Box<dyn std::error::Error + Send + Sync> =
                                            Box::new(e);
                                        boxed_error
                                    }),
                            ))
                            .expect("failed to send eth_sign response");
                    } else {
                        tracing::debug!("sending transaction rejected");
                        response_sender
                            .send(InteractiveResponse::EthSign(Err(Box::new(
                                NexumTuiError::UserRejectedSigning,
                            ))))
                            .expect("failed to send eth_sign response");
                    }
                });
            }
            InteractiveRequest::EthSignTypedData(signer, message) => {
                let (sender, receiver) = oneshot::channel::<(Address, Box<TypedData>, bool)>();
                self.prompt_sender
                    .send(Prompt::EthSignTypedData(signer, message, sender))
                    .expect("failed to send eth_sign_typed_data prompt");
                let wallet = self.wallet_pane.clone();
                tokio::spawn(async move {
                    let (signer, message, should_sign) = receiver
                        .await
                        .expect("failed to receive eth_sign_typed_data response");
                    if should_sign {
                        tracing::debug!("signing and sending transaction now");
                        response_sender
                            .send(InteractiveResponse::EthSignTypedData(
                                wallet
                                    .sign_dynamic_typed_data(Some(signer), &message)
                                    .await
                                    .map_err(|e| {
                                        tracing::error!(?e, "failed to sign typed data");
                                        let boxed_error: Box<dyn std::error::Error + Send + Sync> =
                                            Box::new(e);
                                        boxed_error
                                    }),
                            ))
                            .expect("failed to send eth_sign_typed_data response");
                    } else {
                        tracing::debug!("sending transaction rejected");
                        response_sender
                            .send(InteractiveResponse::EthSignTypedData(Err(Box::new(
                                NexumTuiError::UserRejectedSigning,
                            ))))
                            .expect("failed to send eth_sign_typed_data response");
                    }
                });
            }
        }
    }
}

pub trait HandleEvent {
    fn handle_key(&self, event: &KeyEvent);
}

enum Prompt {
    AccountUnlock(String),
    AccountUnlockInvalidPasswordRetry(String),
    SendTransaction(
        Box<EthereumTypedTransaction<TxEip4844Variant>>,
        oneshot::Sender<(Box<EthereumTypedTransaction<TxEip4844Variant>>, bool)>,
    ),
    EthSign(Address, Bytes, oneshot::Sender<(Address, Bytes, bool)>),
    EthSignTypedData(
        Address,
        Box<TypedData>,
        oneshot::Sender<(Address, Box<TypedData>, bool)>,
    ),
}

#[derive(Debug)]
struct WalletPane {
    is_active: RwLock<bool>,
    accounts: RwLock<Vec<NexumAccount>>,
    list_state: RwLock<ListState>,
    active_wallet_idx: RwLock<Option<usize>>,
    prompt_sender: mpsc::UnboundedSender<Prompt>,
}

impl Widget for &WalletPane {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut list_state = self
            .list_state
            .write()
            .expect("failed to get write lock on list state");
        let list = List::new(
            self.r_accounts()
                .iter()
                .enumerate()
                .map(|(idx, k)| {
                    let name = Text::from(format!(
                        "{} {}",
                        if k.is_locked() { "🔒" } else { "🔓" },
                        k.name()
                    ));
                    if let Some(active_wallet_idx) = *self.r_active_wallet_idx()
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
                .border_style(if *self.r_is_active() {
                    Style::default().fg(Color::Blue)
                } else {
                    Style::default()
                }),
        );
        StatefulWidget::render(list, area, buf, &mut *list_state);
    }
}

macro_rules! delegate_sign_to_account {
    ($method_name:ident, $param_name:ident, $param_type:ty) => {
        async fn $method_name(
            &self,
            from: Option<Address>,
            $param_name: &$param_type,
        ) -> alloy::signers::Result<Signature> {
            if let Some(idx) = {
                // wrapped into a block expresion so the guard can drop after read and before
                // awaiting on sign_hash
                let val = *self.r_active_wallet_idx();
                val
            } {
                // TODO: maybe figure out how to do this without cloning the account
                // right now the accounts shouldn't be that expensive to clone, would just
                // be the signer object
                let account = self.r_accounts()[idx].clone();

                if from.is_some() && from != account.address() {
                    return Err(alloy::signers::Error::Other(Box::new(
                        NexumTuiError::SignerDoesntMatch,
                    )));
                }

                account.$method_name($param_name).await.map_err(|e| {
                    alloy::signers::Error::Other(Box::new(Into::<NexumTuiError>::into(e)))
                })
            } else {
                Err(alloy::signers::Error::Other(Box::new(
                    NexumTuiError::NoActiveWallet,
                )))
            }
        }
    };
}

impl WalletPane {
    fn select_next(&self) {
        let list_state = &mut *self.w_list_state();
        if let Some(selected_idx) = list_state.selected() {
            if selected_idx < self.r_accounts().len() - 1 {
                list_state.select_next();
            } else {
                list_state.select_first();
            }
        } else {
            list_state.select_first();
        }
    }

    fn select_previous(&self) {
        let list_state = &mut *self.w_list_state();
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

    fn w_list_state(&self) -> RwLockWriteGuard<'_, ListState> {
        self.list_state
            .write()
            .expect("failed to get write lock on list state")
    }

    fn r_list_state(&self) -> RwLockReadGuard<'_, ListState> {
        self.list_state
            .read()
            .expect("failed to get read lock on list state")
    }

    fn set_is_active(&self, is_active: bool) {
        *self.w_is_active() = is_active;
    }

    fn set_active_wallet_to_selected_index(&self) -> Option<usize> {
        let list_state = self
            .list_state
            .read()
            .expect("failed to get read lock on list state");
        let idx = list_state.selected();
        *self.w_active_wallet_idx() = idx;
        idx
    }

    fn on_prompt_input(&self, input: String) {
        if let Some(idx) = *self.r_active_wallet_idx()
            && { self.r_accounts()[idx].is_locked() }
        {
            let account = &mut self.w_accounts()[idx];
            if account.try_unlock(input).is_err() {
                self.prompt_sender
                    .send(Prompt::AccountUnlockInvalidPasswordRetry(
                        account.name().to_string(),
                    ))
                    .expect("sending password retry prompt failed");
            }
        }
    }

    fn active_account(&self) -> Option<Address> {
        self.r_active_wallet_idx()
            .map(|idx| self.r_accounts()[idx].address())
            .flatten()
    }

    fn hovered_account(&self) -> Option<Address> {
        self.r_list_state()
            .selected()
            .and_then(|idx| self.r_accounts()[idx].address())
    }

    fn r_accounts(&self) -> RwLockReadGuard<'_, Vec<NexumAccount>> {
        self.accounts
            .read()
            .expect("failed to get read lock on accounts")
    }

    fn w_accounts(&self) -> RwLockWriteGuard<'_, Vec<NexumAccount>> {
        self.accounts
            .write()
            .expect("failed to get write lock on accounts")
    }

    fn r_active_wallet_idx(&self) -> RwLockReadGuard<'_, Option<usize>> {
        self.active_wallet_idx
            .read()
            .expect("failed to get read lock on active wallet idx")
    }

    fn w_active_wallet_idx(&self) -> RwLockWriteGuard<'_, Option<usize>> {
        self.active_wallet_idx
            .write()
            .expect("failed to get write lock on active wallet idx")
    }

    fn r_is_active(&self) -> RwLockReadGuard<'_, bool> {
        self.is_active
            .read()
            .expect("failed to get read lock on is active")
    }

    fn w_is_active(&self) -> RwLockWriteGuard<'_, bool> {
        self.is_active
            .write()
            .expect("failed to get write lock on is active")
    }

    delegate_sign_to_account!(sign_hash, hash, B256);
    delegate_sign_to_account!(sign_message, message, [u8]);
    delegate_sign_to_account!(sign_dynamic_typed_data, payload, TypedData);

    fn add_accounts(&self, to_add: Vec<NexumAccount>) {
        let mut accounts = self.w_accounts();
        accounts.extend(to_add);
    }
}

impl HandleEvent for WalletPane {
    fn handle_key(&self, key: &KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.select_previous(),
            KeyCode::Down | KeyCode::Char('j') => self.select_next(),
            KeyCode::Enter => {
                if let Some(idx) = self.set_active_wallet_to_selected_index()
                    && self.r_accounts()[idx].is_locked()
                {
                    let account = &self.r_accounts()[idx];
                    self.prompt_sender
                        .send(Prompt::AccountUnlock(account.name().to_string()))
                        .expect("sending password prompt request failed");
                }
            }
            _ => {}
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum NexumTuiError {
    /// User rejected signing the transaction, message or typed data
    #[error("user rejected signing")]
    UserRejectedSigning,
    /// Generally means that the signing address and the active wallet address don't match
    #[error("signer doesnt match")]
    SignerDoesntMatch,
    /// No active wallet
    #[error("no active wallet")]
    NoActiveWallet,
    /// Signing error
    #[error("signing error")]
    SigningError(#[from] eyre::Report),
}
