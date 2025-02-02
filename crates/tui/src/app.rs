use alloy_primitives::Address;
use alloy_signer::k256::ecdsa::SigningKey;
use alloy_signer_local::{LocalSigner, PrivateKeySigner};
use ratatui::{
    buffer::Buffer,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListDirection, ListState, Paragraph, StatefulWidget, Widget},
    DefaultTerminal,
};

use crate::widgets::prompts::{PendingPromptsState, PendingPromptsWidget};

#[derive(Debug)]
pub struct App {
    pub should_exit: bool,
    pub rings: RingList,
    pub pane: Pane,
    pub keystore_file: String,
    pub view: View,
    pub prompt_input: String,
    pub signer: Option<LocalSigner<SigningKey>>,
    pub pending_prompts_state: PendingPromptsState,
}

#[derive(Debug)]
enum View {
    KeystorePassword,
    Main,
}

#[derive(PartialEq, Debug)]
enum Pane {
    Rings,
    Keys,
    Actions,
}

#[derive(Debug)]
struct RingList {
    items: Vec<Ring>,
    state: ListState,
}

#[derive(Debug)]
struct Ring {
    name: String,
    addresses: Vec<String>,
    state: ListState,
}

impl Default for App {
    fn default() -> Self {
        Self {
            should_exit: false,
            rings: RingList {
                items: vec![
                    Ring {
                        name: "seed 1".to_string(),
                        addresses: vec!["address 1".to_string(), "address 2".to_string()],
                        state: ListState::default(),
                    },
                    Ring {
                        name: "seed 2".to_string(),
                        addresses: vec!["address 1".to_string(), "address 2".to_string()],
                        state: ListState::default(),
                    },
                    Ring {
                        name: "hote wallet 1".to_string(),
                        addresses: vec!["hot address 1".to_string()],
                        state: ListState::default(),
                    },
                ],

                state: ListState::default(),
            },
            pane: Pane::Rings,
            keystore_file: "".to_string(),
            view: View::KeystorePassword,
            prompt_input: "".to_string(),
            signer: None,
            pending_prompts_state: PendingPromptsState::default(),
        }
    }
}

impl App {
    pub fn run(
        mut self,
        mut terminal: DefaultTerminal,
        keystore_file: String,
    ) -> std::io::Result<()> {
        self.keystore_file = keystore_file;

        while !self.should_exit {
            terminal.draw(|frame| frame.render_widget(&mut self, frame.area()))?;
            if let Event::Key(key) = event::read()? {
                self.handle_key(key);
            };
        }
        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }
        match self.view {
            View::KeystorePassword => match key.code {
                // pasting wont work
                KeyCode::Char(c) => {
                    self.prompt_input.push(c);
                }
                KeyCode::Enter => {
                    if let Ok(signer) =
                        PrivateKeySigner::decrypt_keystore(&self.keystore_file, &self.prompt_input)
                    {
                        self.view = View::Main;
                        self.signer = Some(signer);
                    } else {
                        self.prompt_input.truncate(0);
                    }
                }
                KeyCode::Esc => {
                    self.should_exit = true;
                }
                KeyCode::Backspace => {
                    self.prompt_input.pop();
                }
                _ => {}
            },
            View::Main => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => self.should_exit = true,
                KeyCode::Char('h') | KeyCode::Left => self.prev_pane(),
                KeyCode::Char('l') | KeyCode::Right => self.next_pane(),
                KeyCode::Char('j') | KeyCode::Down => self.next_prompt(),
                KeyCode::Char('k') | KeyCode::Up => self.prev_prompt(),
                _ => {}
            },
        }
    }

    fn prev_pane(&mut self) {
        match self.pane {
            Pane::Rings => self.pane = Pane::Actions,
            Pane::Keys => self.pane = Pane::Rings,
            Pane::Actions => self.pane = Pane::Keys,
        }
    }

    fn next_pane(&mut self) {
        match self.pane {
            Pane::Rings => self.pane = Pane::Keys,
            Pane::Keys => self.pane = Pane::Actions,
            Pane::Actions => self.pane = Pane::Rings,
        }
    }

    fn next_item(&mut self) {
        if self.pane == Pane::Actions {
            return;
        }

        match self.pane {
            Pane::Rings => self.rings.state.select_next(),
            Pane::Keys => {
                if let Some(key) = self.selected_ring() {
                    key.state.select_next()
                }
            }
            _ => {}
        }
    }

    fn prev_item(&mut self) {
        if self.pane == Pane::Actions {
            return;
        }

        match self.pane {
            Pane::Rings => self.rings.state.select_previous(),
            Pane::Keys => {
                if let Some(key) = self.selected_ring() {
                    key.state.select_previous()
                }
            }
            _ => {}
        }
    }

    fn next_prompt(&mut self) {
        if let Some(offset) = self.pending_prompts_state.prompts_list_state.selected() {
            let len = self.pending_prompts_state.prompts.len();
            if offset == len - 1 {
                self.pending_prompts_state.prompts_list_state.select_first();
            } else {
                self.pending_prompts_state.prompts_list_state.select_next();
            }
        } else {
            self.pending_prompts_state.prompts_list_state.select_first();
        }
    }

    fn prev_prompt(&mut self) {
        if let Some(offset) = self.pending_prompts_state.prompts_list_state.selected() {
            if offset == 0 {
                self.pending_prompts_state.prompts_list_state.select_last();
            } else {
                self.pending_prompts_state
                    .prompts_list_state
                    .select_previous();
            }
        } else {
            self.pending_prompts_state.prompts_list_state.select_last();
        }
    }

    fn selected_ring(&mut self) -> Option<&mut Ring> {
        if let Some(offset) = self.rings.state.selected() {
            self.rings.items.get_mut(offset)
        } else {
            None
        }
    }

    fn address(&self) -> Option<Address> {
        self.signer.as_ref().map(|signer| signer.address())
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        buf.reset();
        match self.view {
            View::KeystorePassword => {
                let block = Block::default()
                    .title("Keystore Password: ")
                    .title_alignment(Alignment::Center)
                    .borders(Borders::ALL);

                let input_length = 32;
                let block_area = {
                    let width = input_length + 6;
                    let height = 5;
                    let x = area.width / 2 - width / 2;
                    let y = area.height / 2 - height / 2;

                    Rect::new(x, y, width, height)
                };

                let mut inner = block.inner(block_area);
                inner.y += 1;
                block.render(block_area, buf);
                let password_text = Line::from(vec![Span::styled(
                    format!(
                        "{}{}",
                        "*".repeat(self.prompt_input.len()),
                        " ".repeat((input_length as usize) - self.prompt_input.len())
                    ),
                    Style::default().underlined().black().on_gray(),
                )])
                .alignment(Alignment::Center);
                password_text.render(inner, buf);
            }
            View::Main => {
                let block = Block::default()
                    .title(format!(
                        " Nexum, Address: {} ",
                        self.address().unwrap_or_default()
                    ))
                    .borders(Borders::ALL);
                let inner = block.inner(area);
                block.render(area, buf);
                let pending_prompts_widget = PendingPromptsWidget;
                pending_prompts_widget.render(inner, buf, &mut self.pending_prompts_state);
            }
        }
    }
}
