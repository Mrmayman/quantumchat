use std::{collections::HashMap, sync::Arc};

use iced::widget::operation::AbsoluteOffset;
use tokio::sync::{Mutex, mpsc::UnboundedReceiver};
use whatsmeow_nchat::{ConnId, Jid, MsgId};

use crate::{
    Res,
    state::State,
    storage::Data,
    stylesheet::styles::Theme,
    view::chat_buffer::{DbLoadResult, RMessageCore},
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
    ChatScrollToReply(MsgId),
    ChatScrollToReplyFound(AbsoluteOffset<Option<f32>>),

    ChatBufferLoaded(Res<DbLoadResult>),
    ChatBufferShrink(usize, bool),

    /// Hovered mouse on a message (entered if `true`, exited if `false`)
    ChatMsgHover(MsgId, bool),
    ChatReplyTo(Option<RMessageCore>),
    ChatOpenProfile(Option<Jid>),
}

pub struct App {
    pub id: ConnId,
    pub theme: Theme,
    pub state: State,
    pub db: Data,
    pub message_drafts: HashMap<Jid, MsgDraft>,
    pub typing: HashMap<Jid, Jid>,
}

#[derive(Default)]
pub struct MsgDraft {
    pub text: String,
    pub reply_to: Option<RMessageCore>,
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
