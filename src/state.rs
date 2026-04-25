use std::time::Instant;

use iced::{
    Task,
    widget::{operation::AbsoluteOffset, pane_grid},
};
use whatsmeow_nchat::{Jid, MsgId};

use crate::{
    core::Message,
    view::{chat::ID_MESSAGES, chat_buffer::ChatBuffer},
};

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

    pub msg_hover: Option<MsgId>,
    pub animation_jump: Option<ChatJumpAnimation>,
}

#[derive(Debug, Clone)]
pub struct ChatJumpAnimation {
    pub to_msg: MsgId,
    pub start_time: Instant,
    pub offset: Option<AbsoluteOffset<Option<f32>>>,
}

impl ChatJumpAnimation {
    #[must_use]
    pub fn new(reply_message: MsgId) -> Self {
        Self {
            to_msg: reply_message,
            start_time: Instant::now(),
            offset: None,
        }
    }

    #[must_use]
    pub fn tick(&mut self, current: iced::widget::scrollable::Viewport) -> (Task<Message>, bool) {
        const REPLY_DURATION_MS: f32 = 500.0;

        let progress =
            Instant::now().duration_since(self.start_time).as_millis() as f32 / REPLY_DURATION_MS;
        if progress > 1.0 {
            return (Task::none(), true); // Finished
        }
        let Some(offset) = self.offset else {
            return (Task::none(), false); // Didn't start yet
        };

        (
            iced::widget::operation::scroll_to(
                ID_MESSAGES,
                AbsoluteOffset {
                    x: offset.x,
                    y: offset
                        .y
                        .map(|y| ease_out(progress, current.absolute_offset().y, y)),
                },
            ),
            false,
        )
    }
}

fn ease_out(step: f32, start: f32, end: f32) -> f32 {
    let t = step.clamp(0.0, 1.0);
    let eased = 1.0 - (1.0 - t) * (1.0 - t); // quadratic ease-out
    start + (end - start) * eased
}
