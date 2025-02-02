use core::fmt;

use alloy_primitives::{Address, Bytes, FixedBytes, Uint};
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Position},
    prelude::{Buffer, Rect},
    style::{Style, Stylize},
    text::{Line, Text},
    widgets::{
        Block, Borders, Cell, List, ListDirection, ListItem, ListState, Paragraph, Row,
        StatefulWidget, Table, TableState, Widget,
    },
};

pub struct TransactionPromptWidget;
pub struct TransactionPromptState {
    active_field_idx: usize,
    fields: Vec<Field>,
}

impl Default for TransactionPromptState {
    fn default() -> Self {
        Self {
            active_field_idx: Default::default(),
            fields: vec![
                Field {
                    name: "to".into(),
                    value: FieldValue::Address(Address::ZERO),
                },
                Field {
                    name: "gas_price".into(),
                    value: FieldValue::Number(0),
                },
                Field {
                    name: "gas".into(),
                    value: FieldValue::Number(0),
                },
                Field {
                    name: "nonce".into(),
                    value: FieldValue::Number(0),
                },
                Field {
                    name: "data".into(),
                    value: FieldValue::Bytes(Bytes::default()),
                },
            ],
        }
    }
}

impl TransactionPromptState {
    pub fn max_field_name_length(&self) -> usize {
        self.fields
            .iter()
            .map(|f| f.name.len())
            .max()
            .unwrap_or_default()
    }
}

pub enum FieldValue {
    Address(Address),
    Boolean(bool),
    Number(u64),
    Bytes(Bytes),
}

impl std::fmt::Display for FieldValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldValue::Address(address) => write!(f, "{}", address),
            FieldValue::Boolean(value) => write!(f, "{}", value),
            FieldValue::Number(value) => write!(f, "{}", value),
            FieldValue::Bytes(bytes) => write!(f, "{}", bytes),
        }
    }
}

pub struct Field {
    name: String,
    value: FieldValue,
}

#[derive(Clone, Debug)]
pub struct ConnectionPrompt {
    pub origin: String,
}

#[derive(Clone, Debug)]
pub struct TransactionRequestPrompt {
    pub to: Address,
    pub gas_limit: u64,
    pub gas_price: u64,
    pub value: Uint<256, 4>,
    pub data: Bytes,
}

#[derive(Clone, Debug)]
pub struct TypedSignatureRequestPrompt {}

#[derive(Clone, Debug)]
pub struct RawSignatureRequestPrompt {
    hash: FixedBytes<32>,
}

#[derive(Clone, Debug)]
pub enum Prompt {
    Connection(ConnectionPrompt),
    TransactionRequest(TransactionRequestPrompt),
    TypedSignatureRequest(TypedSignatureRequestPrompt),
    RawSignatureRequest(RawSignatureRequestPrompt),
}

impl Prompt {
    fn lines(&self) -> Vec<Line> {
        match self {
            Prompt::Connection(c) => {
                vec!["Connect to".into(), c.origin.clone().into()]
            }
            Prompt::TransactionRequest(r) => {
                let min_len = ["to", "gas_limit", "gas_price", "value", "data"]
                    .iter()
                    .map(|f| f.len())
                    .max()
                    .unwrap_or_default()
                    + 2;

                let text = format!(
                    "to{}: {}\ngas_limit{}: {}\ngas_price{}: {}\nvalue{}: {}\ndata{}: {}",
                    " ".repeat(min_len - "to".len()),
                    r.to,
                    " ".repeat(min_len - "gas_limit".len()),
                    r.gas_limit,
                    " ".repeat(min_len - "gas_price".len()),
                    r.gas_price,
                    " ".repeat(min_len - "value".len()),
                    r.value,
                    " ".repeat(min_len - "data".len()),
                    r.data
                );
                let lines = text.split("\n").collect::<Vec<_>>();
                // println!(
                //     "lnes: {:?}",
                //     lines.iter().map(|f| f.len()).collect::<Vec<_>>()
                // );
                lines.into_iter().map(|f| f.to_owned().into()).collect()
            }
            _ => vec![],
        }
    }
}

const PIPE_CORNER_TOP_LEFT: char = '┌';
const PIPE_CORNER_BOTTOM_LEFT: char = '└';
const PIPE_CORNER_TOP_RIGHT: char = '┐';
const PIPE_CORNER_BOTTOM_RIGHT: char = '┘';

const PIPE_HORIZONTAL: char = '─';
const PIPE_VERTICAL: char = '│';

const PIPE_MID_LEFT: char = '├';
const PIPE_MID_RIGHT: char = '┤';
const PIPE_MID_TOP: char = '┬';
const PIPE_MID_BOTTOM: char = '┴';

const PIPE_MID: char = '┼';

impl Widget for Prompt {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let lines = self.lines();
        let max_width = lines.iter().map(|f| f.width()).max().unwrap_or_default() as u16;

        // needs to be odd to get an equal split on the actions button area
        let min_width = (23).max(max_width + 2);
        let prompt_content_height = lines.len() as u16;
        let actions_content_height = 1;
        let height = prompt_content_height + actions_content_height + 3;
        let prompt_area = Rect::new(area.x + 1, area.y + 1, min_width, prompt_content_height);
        let actions_area = Rect::new(
            area.x + 1,
            area.y + prompt_content_height + 2,
            min_width,
            actions_content_height,
        );
        let outer_area = Rect::new(area.x, area.y, min_width + 3, height);

        // draw the box
        for (yidx, _) in (0..height).enumerate() {
            for (xidx, _) in (0..min_width).enumerate() {
                let yidx = yidx as u16;
                let xidx = xidx as u16;
                let cell_position = Position {
                    x: area.x + xidx,
                    y: area.y + yidx,
                };
                let cell = buf.cell_mut(cell_position).expect("!cell");
                let cell_char = if yidx == 0 && xidx == 0 {
                    PIPE_CORNER_TOP_LEFT
                } else if yidx == 0 && xidx == min_width - 1 {
                    PIPE_CORNER_TOP_RIGHT
                } else if yidx == height - 1 && xidx == 0 {
                    PIPE_CORNER_BOTTOM_LEFT
                } else if yidx == height - 1 && xidx == min_width - 1 {
                    PIPE_CORNER_BOTTOM_RIGHT
                } else if yidx == prompt_content_height + 1 && xidx == 0 {
                    PIPE_MID_LEFT
                } else if yidx == prompt_content_height + 1 && xidx == min_width - 1 {
                    PIPE_MID_RIGHT
                } else if yidx == prompt_content_height + 1 && xidx == (min_width - 1) / 2 {
                    PIPE_MID_TOP
                } else if yidx == height - 1 && xidx == (min_width - 1) / 2 {
                    PIPE_MID_BOTTOM
                } else if yidx == 0 || yidx == height - 1 || yidx == prompt_content_height + 1 {
                    PIPE_HORIZONTAL
                } else if xidx == 0
                    || xidx == min_width - 1
                    || ((xidx == (min_width - 1) / 2) && yidx > prompt_content_height + 1)
                {
                    PIPE_VERTICAL
                } else {
                    ' '
                };
                cell.set_char(cell_char);
            }
        }

        // render the prompt
        for (line_idx, line) in lines.iter().enumerate() {
            let line_area = Rect::new(
                prompt_area.x,
                prompt_area.y + (line_idx as u16),
                prompt_area.width,
                1,
            );
            line.render(line_area, buf);
        }

        // render the actions
        let accept_area = Rect::new(
            actions_area.x,
            actions_area.y,
            (actions_area.width - 1) / 2,
            actions_area.height,
        );
        Text::from("[a]ccept").centered().render(accept_area, buf);
        let reject_area = Rect::new(
            actions_area.x + ((actions_area.width - 1) / 2),
            actions_area.y,
            (actions_area.width - 1) / 2,
            actions_area.height,
        );
        Text::from("[r]eject").centered().render(reject_area, buf);

        // let max_len = self
        //     .lines()
        //     .iter()
        //     .map(|l| l.width())
        //     .max()
        //     .unwrap_or_default();
        //
        // let block_area = Rect::new(
        //     area.x,
        //     area.y,
        //     ((max_len as u16) + 2).max(20),
        //     (self.lines().len() as u16) + 2,
        // );
        //
        // let block = Block::default().borders(Borders::ALL);
        // block.render(block_area, buf);
        //
        // let mut line_area: Rect = Rect::default();
        // for (idx, line) in self.lines().iter().enumerate() {
        //     line_area = Rect::new(
        //         area.x + 1,
        //         area.y + 1 + (idx as u16),
        //         line.width() as u16,
        //         1,
        //     );
        //     line.render(line_area, buf);
        // }
        //
        // let buttons_area = Rect::new(block_area.x, line_area.y + 2, block_area.width, 5);
        //
        // if let [reject, accept] = Layout::default()
        //     .constraints(vec![Constraint::Fill(1), Constraint::Fill(1)])
        //     .direction(Direction::Horizontal)
        //     .split(buttons_area)[..]
        // {
        //     Block::default().borders(Borders::ALL).render(reject, buf);
        //     Text::from("Reject").centered().render(
        //         reject.inner(Margin {
        //             horizontal: 1,
        //             vertical: 1,
        //         }),
        //         buf,
        //     );
        //
        //     Block::default()
        //         .borders(Borders::TOP | Borders::BOTTOM | Borders::RIGHT)
        //         .render(accept, buf);
        //     Text::from("Accept").centered().render(
        //         accept.inner(Margin {
        //             horizontal: 1,
        //             vertical: 1,
        //         }),
        //         buf,
        //     );
        // }
        //
        // // let inner = block.inner(block_area);
    }
}

#[derive(Default, Debug)]
pub struct PendingPromptsState {
    pub prompts: Vec<Prompt>,
    pub prompts_list_state: ListState,
}

pub struct PendingPromptsWidget;

impl StatefulWidget for PendingPromptsWidget {
    type State = PendingPromptsState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // let block = Block::default()
        //     .title("Transaction Prompt")
        //     .borders(Borders::ALL);
        // let inner = block.inner(area);
        // block.render(area, buf);
        // let target_len = state.max_field_name_length();
        // let para = Paragraph::new(
        //     state
        //         .fields
        //         .iter()
        //         .map(|f| {
        //             format!(
        //                 "{}{} : {}",
        //                 f.name,
        //                 " ".repeat(target_len - f.name.len()),
        //                 f.value
        //             )
        //         })
        //         .collect::<Vec<_>>()
        //         .join("\n"),
        // );
        // para.render(inner, buf);
        // let mut table_state = TableState::default().with_selected(Some(0));
        // let rows = state
        //     .fields
        //     .iter()
        //     .map(|f| {
        //         Row::new(vec![
        //             Cell::new(f.name.clone()),
        //             Cell::new(f.value.to_string()),
        //         ])
        //     })
        //     .collect::<Vec<_>>();
        // StatefulWidget::render(
        //     Table::new(rows, [Constraint::Length(7), Constraint::Length(30)])
        //         .block(Block::new())
        //         .header(Row::new(vec!["Qty", "Ingredient"]))
        //         .row_highlight_style(Style::new().light_yellow()),
        //     area,
        //     buf,
        //     &mut table_state,
        // );

        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Length(5), Constraint::Fill(5)]);
        if let [selector, viewer] = layout.split(area.inner(Margin {
            horizontal: 1,
            vertical: 1,
        }))[..]
        {
            let selector_block = Block::default().borders(Borders::RIGHT);
            let selector_block_inner = selector_block.inner(selector);
            selector_block.render(selector, buf);

            let selector_list = List::new(
                (1..=state.prompts.len())
                    .collect::<Vec<_>>()
                    .iter()
                    .map(|i| ListItem::new(Text::from(i.to_string()).right_aligned()))
                    .collect::<Vec<_>>(),
            )
            .direction(ListDirection::TopToBottom)
            .highlight_style(Style::new().on_gray().black());
            StatefulWidget::render(
                selector_list,
                selector_block_inner,
                buf,
                &mut state.prompts_list_state,
            );

            if let Some(selected) = state.prompts_list_state.selected() {
                let prompt = state.prompts[selected].clone();
                prompt.render(viewer, buf);
            }
        } else {
            Paragraph::new("layout split error").render(area, buf);
        }
    }
}
