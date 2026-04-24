use std::{collections::HashMap, sync::Arc};

use tokio::sync::{Mutex, mpsc::UnboundedReceiver};
use whatsmeow_nchat::{ConnId, Jid};

use crate::{
    Res, state::State, storage::Data, stylesheet::styles::Theme, view::chat_buffer::DbLoadResult,
};

type Recv = Arc<Mutex<UnboundedReceiver<whatsmeow_nchat::Event>>>;

#[derive(Debug, Clone)]
pub enum Message {
    Nothing,
    Done(Res),

    Connected(Res<(ConnId, Recv)>),
    CoreTick,
    WEvent(whatsmeow_nchat::Event),
    CoreEvent(iced::event::Event, iced::event::Status),

    OpenMainMenu,
    SidebarResize(f32),
    ChatSelected(Jid),

    /// Load more messages, reached edge of scrollable.
    /// `.0` represents whether scrolled up (`true`) or down.
    ChatScrollLazyLoad(bool),
    ChatScrolledView(iced::widget::scrollable::Viewport),
    ChatMessageInput(String),
    ChatSend,

    ChatBufferLoaded(Res<DbLoadResult>),
    ChatBufferShrink(usize, bool),
}

pub struct App {
    pub id: ConnId,
    pub theme: Theme,
    pub state: State,
    pub db: Data,
    pub message_drafts: HashMap<Jid, String>,
    pub typing: HashMap<Jid, Jid>,

    pub tick_timer: u128,
}

pub trait IntoStringError<T> {
    #[allow(clippy::missing_errors_doc)]
    fn strerr(self) -> Result<T, String>;
}

impl<T, E: ToString> IntoStringError<T> for Result<T, E> {
    fn strerr(self) -> Result<T, String> {
        self.map_err(|err| err.to_string())
    }
}

#[macro_export]
macro_rules! jid {
    ($s:expr) => {
        ::whatsmeow_nchat::Jid::parse(&$s).ok_or_else(|| {
            format!(
                "JID parse error ({}:{}:{})",
                file!(),
                line!(),
                ::core::column!()
            )
        })?
    };
}
