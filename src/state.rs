use iced::widget::pane_grid;
use whatsmeow_nchat::Jid;

use crate::view::chat_buffer::ChatBuffer;

pub enum State {
    Loading,
    Login(MenuLogin),
    Chats(MenuChats, Option<ChatUI>),
    Error(String),
    // Settings
}

pub struct MenuLogin {
    pub code: String,
    pub qr_code: Option<iced::widget::qr_code::Data>,
}

impl MenuLogin {
    pub fn new(code: String, is_qr: bool) -> Result<Self, iced::widget::qr_code::Error> {
        Ok(Self {
            qr_code: is_qr
                .then(|| iced::widget::qr_code::Data::new(&code))
                .transpose()?,
            code,
        })
    }
}

pub struct MenuChats {
    pub sidebar_grid_state: pane_grid::State<bool>,
    pub sidebar_split: Option<pane_grid::Split>,

    pub opened_profile: Option<Jid>,
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
            opened_profile: None,
        }
    }
}

pub struct ChatUI {
    pub selected: Jid,
    pub chat_buffer: ChatBuffer,
}
