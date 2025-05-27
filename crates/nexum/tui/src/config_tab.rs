use std::sync::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};

use alloy::primitives::Address;
use alloy_chains::NamedChain;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Layout},
    prelude::{Buffer, Rect},
    style::{Style, Stylize},
    widgets::{Block, List, ListState, Padding, Row, StatefulWidget, Table, Widget},
};

use crate::{config::Config, HandleEvent};

pub struct ConfigTab {
    config: Config,
    config_list_state: Mutex<ListState>,
    origin_connections_collapsed: RwLock<bool>,
    labels_collapsed: RwLock<bool>,
}

#[derive(Debug)]
enum ConfigListItemType {
    Rpcs,
    OriginConnections(Address),
    OriginConnectionsMeta,
    Labels(NamedChain),
    LabelsMeta,
    Meta,
}

impl ConfigTab {
    pub fn new(config: Config) -> Self {
        let mut list_state = ListState::default();
        list_state.select_first();

        Self {
            config,
            config_list_state: Mutex::new(list_state.clone()),
            origin_connections_collapsed: false.into(),
            labels_collapsed: false.into(),
        }
    }

    fn list_len(&self) -> usize {
        3 + if *self.r_origin_connections_collapsed() {
            0
        } else {
            self.config.origin_connections.len()
        } + if *self.r_labels_collapsed() {
            0
        } else {
            self.config.labels.len()
        }
    }

    fn origin_connections_offset(&self) -> usize {
        1
    }

    fn labels_offset(&self) -> usize {
        2 + if *self.r_origin_connections_collapsed() {
            0
        } else {
            self.config.origin_connections.len()
        }
    }

    fn item_at(&self, idx: usize) -> ConfigListItemType {
        let labels_offset = self.labels_offset();
        let origin_connections_offset = self.origin_connections_offset();
        let list_len = self.list_len();

        match idx {
            0 => ConfigListItemType::Rpcs,
            idx if idx == origin_connections_offset => ConfigListItemType::OriginConnectionsMeta,
            idx if idx == labels_offset => ConfigListItemType::LabelsMeta,
            idx if idx < labels_offset => ConfigListItemType::OriginConnections(
                *self
                    .config
                    .origin_connections
                    .keys()
                    .nth(idx - origin_connections_offset - 1)
                    .expect("idx is out of bounds"),
            ),
            idx if idx < list_len => ConfigListItemType::Labels(
                *self
                    .config
                    .labels
                    .keys()
                    .nth(idx - labels_offset - 1)
                    .expect("idx is out of bounds"),
            ),
            _ => ConfigListItemType::Meta,
        }
    }

    fn select_next_config_type(&self) {
        let list_state = &mut *self
            .config_list_state
            .lock()
            .expect("failed to get config list state");
        if let Some(selected_idx) = list_state.selected() {
            if selected_idx < self.list_len() - 1 {
                list_state.select_next();
            } else {
                list_state.select_first();
            }
        } else {
            list_state.select_first();
        }
    }

    fn select_previous_config_type(&self) {
        let list_state = &mut *self
            .config_list_state
            .lock()
            .expect("failed to get config list state");
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

    fn r_origin_connections_collapsed(&self) -> RwLockReadGuard<bool> {
        self.origin_connections_collapsed
            .read()
            .expect("failed to get read lock on origin_connections_collapsed")
    }

    fn w_origin_connections_collapsed(&self) -> RwLockWriteGuard<bool> {
        self.origin_connections_collapsed
            .write()
            .expect("failed to get write lock on origin_connections_collapsed")
    }

    fn r_labels_collapsed(&self) -> RwLockReadGuard<bool> {
        self.labels_collapsed
            .read()
            .expect("failed to get read lock on labels_collapsed")
    }

    fn w_labels_collapsed(&self) -> RwLockWriteGuard<bool> {
        self.labels_collapsed
            .write()
            .expect("failed to get write lock on labels_collapsed")
    }
}

impl Widget for &ConfigTab {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let mut list_items =
            Vec::with_capacity(3 + self.config.origin_connections.len() + self.config.labels.len());
        list_items.push("RPCs".to_string());

        if *self.r_origin_connections_collapsed() {
            list_items.push("▶ Origin Connections".to_string())
        } else {
            list_items.push("▼ Origin Connections".to_string());
            for addr in self.config.origin_connections.keys() {
                list_items.push(format!("  {addr}"));
            }
        }

        if *self.r_labels_collapsed() {
            list_items.push("▶ Labels".to_string())
        } else {
            list_items.push("▼ Labels".to_string());
            for chain in self.config.labels.keys() {
                list_items.push(format!("  {chain}"));
            }
        }
        assert!(
            list_items.len() == self.list_len(),
            "list_items.len() is wrong"
        );

        let list = List::new(list_items)
            .highlight_style(Style::default().reversed())
            .highlight_symbol("> ")
            .block(Block::bordered());
        let [left_area, right_area] =
            Layout::horizontal(vec![Constraint::Ratio(1, 5), Constraint::Ratio(4, 5)]).areas(area);

        StatefulWidget::render(
            list,
            left_area,
            buf,
            &mut self
                .config_list_state
                .lock()
                .expect("failed to get config list state"),
        );

        if let Some(idx) = self
            .config_list_state
            .lock()
            .expect("failed to get config list state")
            .selected()
        {
            let item = self.item_at(idx);
            match item {
                ConfigListItemType::Rpcs => {
                    let table = Table::new(
                        self.config
                            .rpcs
                            .iter()
                            .map(|(k, v)| Row::new(vec![k.to_owned(), v.to_string()]))
                            .collect::<Vec<_>>(),
                        vec![Constraint::Percentage(20), Constraint::Percentage(80)],
                    )
                    .column_spacing(1)
                    .header(
                        Row::new(vec!["Name", "URL"])
                            .style(Style::default().bold())
                            .bottom_margin(1),
                    )
                    .block(
                        Block::bordered()
                            .title("RPCs")
                            .padding(Padding::new(1, 0, 0, 0)),
                    );
                    Widget::render(table, right_area, buf);
                }
                ConfigListItemType::OriginConnections(addr) => {
                    let table = Table::new(
                        self.config
                            .origin_connections
                            .get(&addr)
                            .unwrap()
                            .iter()
                            .map(|(k, v)| Row::new(vec![k.to_string(), v.to_string()]))
                            .collect::<Vec<_>>(),
                        vec![Constraint::Percentage(20), Constraint::Percentage(80)],
                    )
                    .column_spacing(1)
                    .header(
                        Row::new(vec!["Origin", "Allowed"])
                            .style(Style::default().bold())
                            .bottom_margin(1),
                    )
                    .block(
                        Block::bordered()
                            .title("Origin Connections")
                            .padding(Padding::new(1, 0, 0, 0)),
                    );
                    Widget::render(table, right_area, buf);
                }
                ConfigListItemType::Labels(chain) => {
                    let table = Table::new(
                        self.config
                            .labels
                            .get(&chain)
                            .unwrap()
                            .iter()
                            .map(|(k, v)| Row::new(vec![k.to_string(), v.to_string()]))
                            .collect::<Vec<_>>(),
                        vec![Constraint::Length(42), Constraint::Percentage(100)],
                    )
                    .column_spacing(1)
                    .header(
                        Row::new(vec!["Address", "Label"])
                            .style(Style::default().bold())
                            .bottom_margin(1),
                    )
                    .block(
                        Block::bordered()
                            .title("Labels")
                            .padding(Padding::new(1, 0, 0, 0)),
                    );
                    Widget::render(table, right_area, buf);
                }
                _ => {
                    Widget::render(Block::bordered(), right_area, buf);
                }
            }
        }
    }
}

impl HandleEvent for ConfigTab {
    fn handle_key(&self, event: &KeyEvent) {
        match event.code {
            KeyCode::Up | KeyCode::Char('k') => self.select_previous_config_type(),
            KeyCode::Down | KeyCode::Char('j') => self.select_next_config_type(),
            KeyCode::Enter => {
                if let Some(idx) = self
                    .config_list_state
                    .lock()
                    .expect("failed to get config list state")
                    .selected()
                {
                    let item = self.item_at(idx);
                    match item {
                        ConfigListItemType::OriginConnectionsMeta => {
                            let new_value = {
                                let prev = *self.r_origin_connections_collapsed();
                                !prev
                            };
                            *self.w_origin_connections_collapsed() = new_value;
                        }
                        ConfigListItemType::LabelsMeta => {
                            let new_value = {
                                let prev = *self.r_labels_collapsed();
                                !prev
                            };
                            *self.w_labels_collapsed() = new_value;
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}
