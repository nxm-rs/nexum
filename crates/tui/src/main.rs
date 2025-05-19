#![feature(let_chains)]

use std::{
    fs::OpenOptions,
    path::PathBuf,
    sync::{RwLock, RwLockWriteGuard},
    time::Duration,
};

use crossterm::event::{Event, EventStream, KeyCode, KeyEvent};
use eyre::OptionExt;
use futures::StreamExt;
use ratatui::{
    layout::{Constraint, Layout},
    prelude::{Buffer, Rect},
    style::{Color, Style, Stylize},
    text::Text,
    widgets::{Block, Borders, FrameExt, List, ListState, StatefulWidget, Widget},
    DefaultTerminal, Frame,
};
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
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(tui_logger)
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let terminal = ratatui::init();
    let app_result = App::default().run(terminal).await;
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
}

impl Default for App {
    fn default() -> Self {
        let mut list_state = ListState::default();
        list_state.select_first();
        Self {
            should_quit: false,
            active_pane: AppPane::Wallet,
            wallet_pane: WalletPane {
                is_active: true,
                keystores: load_keystores().unwrap_or_default(),
                list_state: RwLock::new(list_state),
                active_wallet_idx: None,
            },
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

        Ok(())
    }

    fn handle_event(&mut self, event: &Event) {
        if let Some(key) = event.as_key_press_event() {
            match (&self.active_pane, key.code) {
                (_, KeyCode::Char('q') | KeyCode::Esc) => self.should_quit = true,
                (_, KeyCode::Tab) => {
                    self.active_pane = self.active_pane.next();
                    self.wallet_pane
                        .set_is_active(matches!(self.active_pane, AppPane::Wallet));
                }
                (AppPane::Wallet, _) => self.wallet_pane.handle_key(&key),
                _ => {}
            }
        }
    }
}

/// Returns all the keystore file paths in the foundry keystore directory.
fn load_keystores() -> eyre::Result<Vec<PathBuf>> {
    let home_dir = std::env::home_dir().ok_or_eyre("home directory not found")?;
    let foundry_keystore_dir = home_dir.join(".foundry/keystores");
    let foundry_keystore_files = std::fs::read_dir(foundry_keystore_dir)?;

    Ok(foundry_keystore_files
        .into_iter()
        .filter_map(|f| f.ok().map(|f| f.path()))
        .collect::<Vec<_>>())
}

trait HandleEvent {
    fn handle_key(&mut self, event: &KeyEvent);
}

struct WalletPane {
    is_active: bool,
    keystores: Vec<PathBuf>,
    list_state: RwLock<ListState>,
    active_wallet_idx: Option<usize>,
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
                .filter_map(|f| f.file_name().map(|f| f.to_str().map(|s| s.to_owned())))
                .flatten()
                .enumerate()
                .map(|(idx, k)| {
                    if let Some(active_wallet_idx) = self.active_wallet_idx
                        && idx == active_wallet_idx
                    {
                        Text::from(k).style(Style::default().bold().fg(Color::Blue))
                    } else {
                        Text::from(k)
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

    fn set_active_wallet_to_selected_index(&mut self) {
        let list_state = self
            .list_state
            .read()
            .expect("failed to get read lock on list state");
        if let Some(selected_idx) = list_state.selected() {
            self.active_wallet_idx = Some(selected_idx);
        }
    }
}

impl HandleEvent for WalletPane {
    fn handle_key(&mut self, key: &KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.select_previous(),
            KeyCode::Down | KeyCode::Char('j') => self.select_next(),
            KeyCode::Enter => self.set_active_wallet_to_selected_index(),
            _ => {}
        }
    }
}
