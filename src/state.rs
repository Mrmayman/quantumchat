use std::time::Duration;

use iced::widget::pane_grid;

use crate::storage::contact::Jid;

pub enum State {
    Login(MenuLogin),
    Chats(MenuChats, Option<ChatUI>),
    Error(String),
    // Settings
}

pub struct MenuLogin {
    pub timeout: Duration,
    pub qr_code: iced::widget::qr_code::Data,
    pub initial_time: std::time::Instant,
}

impl MenuLogin {
    pub fn new(code: String, timeout: Duration) -> Result<Self, iced::widget::qr_code::Error> {
        Ok(Self {
            qr_code: iced::widget::qr_code::Data::new(&code)?,
            timeout,
            initial_time: std::time::Instant::now(),
        })
    }
}

pub struct MenuChats {
    pub sidebar_grid_state: pane_grid::State<bool>,
    pub sidebar_split: Option<pane_grid::Split>,
}

impl MenuChats {
    pub fn new() -> Self {
        let (mut sidebar_grid_state, pane) = pane_grid::State::new(true);
        let sidebar_split = if let Some((_, split)) =
            sidebar_grid_state.split(pane_grid::Axis::Vertical, pane, false)
        {
            sidebar_grid_state.resize(split, 0.33);
            Some(split)
        } else {
            None
        };

        Self {
            sidebar_grid_state,
            sidebar_split,
        }
    }
}

pub struct ChatUI {
    pub selected: Jid,
}
