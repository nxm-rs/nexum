use ratatui::{
    buffer::Buffer,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Constraint, Direction, Layout, Rect},
    prelude::Stylize,
    style::Style,
    widgets::{Block, Borders, List, ListDirection, ListState, Paragraph, StatefulWidget, Widget},
    DefaultTerminal,
};

#[derive(Debug)]
struct App {
    should_exit: bool,
    rings: RingList,
    pane: Pane,
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
        }
    }
}

impl App {
    fn run(mut self, mut terminal: DefaultTerminal) -> std::io::Result<()> {
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
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_exit = true,
            KeyCode::Char('h') | KeyCode::Left => self.prev_pane(),
            KeyCode::Char('l') | KeyCode::Right => self.next_pane(),
            KeyCode::Char('j') | KeyCode::Down => self.next_item(),
            KeyCode::Char('k') | KeyCode::Up => self.prev_item(),
            _ => {}
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

    fn selected_ring(&mut self) -> Option<&mut Ring> {
        if let Some(offset) = self.rings.state.selected() {
            self.rings.items.get_mut(offset)
        } else {
            None
        }
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title(format!(" Nexum, Pane: {:?} ", self.pane))
            .borders(Borders::ALL);
        let inner = block.inner(area);
        block.render(area, buf);

        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Fill(1),
                Constraint::Fill(1),
                Constraint::Fill(3),
            ])
            .split(inner);
        let [ring, key, window] = layout[..] else {
            panic!("incorrect layout")
        };

        let ring_list = List::new(
            self.rings
                .items
                .iter()
                .map(|item| item.name.as_str())
                .collect::<Vec<_>>(),
        )
        .highlight_style(Style::new().black().on_white())
        .highlight_symbol(">>")
        .direction(ListDirection::TopToBottom)
        .block(Block::default().borders(Borders::ALL));

        Paragraph::new(format!("selected offset: {}", self.rings.state.offset())).render(key, buf);
        StatefulWidget::render(ring_list, ring, buf, &mut self.rings.state);

        if let Some(ring) = self.selected_ring() {
            let keys_list = List::new(
                ring.addresses
                    .iter()
                    .map(|item| item.as_str())
                    .collect::<Vec<_>>(),
            )
            .highlight_style(Style::new().black().on_white())
            .highlight_symbol(">>")
            .direction(ListDirection::TopToBottom)
            .block(Block::default().borders(Borders::ALL));
            StatefulWidget::render(keys_list, key, buf, &mut ring.state);
        }
    }
}

fn main() -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    terminal.clear()?;
    let app_result = App::default().run(terminal);
    ratatui::restore();
    app_result
}
