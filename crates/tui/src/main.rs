use std::{fs::OpenOptions, path::PathBuf, time::Duration};

use crossterm::event::{Event, EventStream, KeyCode};
use eyre::OptionExt;
use futures::StreamExt;
use ratatui::{
    layout::{Constraint, Layout},
    style::Stylize,
    text::Line,
    widgets::{Block, Borders},
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
    tracing::info!("info");
    tracing::warn!("warn");
    tracing::error!("error");
    tracing::debug!("debug");
    tracing::trace!("trace");

    let terminal = ratatui::init();
    let app_result = App::default().run(terminal).await;
    ratatui::restore();
    app_result
}

#[derive(Default)]
struct App {
    pub should_quit: bool,
}

impl App {
    const FRAMES_PER_SECOND: u64 = 10;

    pub async fn run(mut self, mut terminal: DefaultTerminal) -> eyre::Result<()> {
        let period = Duration::from_secs_f32(1.0 / (Self::FRAMES_PER_SECOND as f32));
        let mut interval = tokio::time::interval(period);
        let mut events = EventStream::new();

        while !self.should_quit {
            tokio::select! {
                _ = interval.tick() => { terminal.draw(|f| self.render(f))?; },
                Some(Ok(event)) = events.next() => self.handle_event(&event),
            }
        }
        Ok(())
    }

    fn render(&self, frame: &mut Frame) {
        let vertical = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]);
        let [title_area, body_area] = vertical.areas(frame.area());
        let title = Line::from("Ratatui async example").centered().bold();
        let block = Block::new().borders(Borders::ALL);
        frame.render_widget(title, title_area);
        frame.render_widget(block, body_area);
    }

    fn handle_event(&mut self, event: &Event) {
        if let Some(key) = event.as_key_press_event() {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
                _ => {}
            }
        }
    }
}
