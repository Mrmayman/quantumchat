use iced::{
    Task,
    widget::{self, operation, selector},
};
use tokio::task::spawn_blocking;
use whatsmeow_nchat::MsgId;

use crate::{
    core::{App, IntoStringError, Message},
    state::{ChatJumpAnimation, State},
};

impl App {
    pub fn send_msg(&mut self) -> Task<Message> {
        let State::Chats(_, Some(chat)) = &mut self.state else {
            return Task::none();
        };
        let chat_id = chat.selected.clone();
        let Some(contents) = self.message_drafts.remove(&chat_id) else {
            return Task::none();
        };
        if contents.text.is_empty() {
            return Task::none();
        }
        let id = self.id;

        Task::perform(
            spawn_blocking(move || {
                let reply = contents.reply_to.map(|n| whatsmeow_nchat::QuotedMessage {
                    sender: n.sender,
                    contents: n.text.into_iter().fold(String::new(), |mut accum, span| {
                        accum.push_str(&span.text);
                        accum
                    }),
                    message_id: n.id,
                });
                whatsmeow_nchat::send_message(
                    id,
                    &chat_id,
                    &contents.text,
                    reply.as_ref(),
                    None::<(&std::path::Path, _)>,
                    None,
                )
                .strerr()
            }),
            |n| Message::Done(n.strerr().and_then(|n| n)),
        )
    }

    pub fn scroll_to_reply(&mut self, msg_id: MsgId) -> Task<Message> {
        let widget_id = format!("msg:{}", msg_id.0);
        if let State::Chats(_, Some(ui)) = &mut self.state {
            ui.animation_jump = Some(ChatJumpAnimation::new(msg_id));
        }
        scroll_into_view("messages", widget_id)
    }

    pub fn scroll_to_reply_done(
        &mut self,
        offset: iced::widget::operation::AbsoluteOffset<Option<f32>>,
    ) {
        if let State::Chats(_, Some(chat)) = &mut self.state
            && let Some(anim) = &mut chat.animation_jump
        {
            let viewport = chat.chat_buffer.scroll;
            let viewport_height = viewport.bounds().height - 60.0;
            if offset.y.is_some_and(|y| {
                let delta = y - viewport.absolute_offset().y;
                delta > 0.0 && delta < viewport_height
            }) {
                // Skip if already on screen
                return;
            }

            anim.offset = Some(offset);
        }
    }
}

// Adapted from generic-daw:
// https://github.com/generic-daw/generic-daw/blob/main/generic_daw_gui/src/operation.rs
//
// Copyright (C) 2026 edwloef
// Licensed under the GNU General Public License v3.0
pub fn scroll_into_view(
    scrollable: impl Into<widget::Id>,
    child: impl Into<widget::Id>,
) -> Task<Message> {
    let scrollable = scrollable.into();
    let child = child.into();

    selector::find(scrollable.clone())
        .and_then(move |s| selector::find(child.clone()).map(move |c| c.map(|c| (s.clone(), c))))
        .and_then(move |(s, c)| {
            let selector::Target::Scrollable { translation, .. } = s else {
                panic!();
            };

            let offset = operation::AbsoluteOffset {
                x: c.visible_bounds()
                    .is_none_or(|vb| vb.width != c.bounds().width)
                    .then_some(
                        c.bounds().x - s.bounds().x
                            + if c.bounds().x - s.bounds().x < translation.x {
                                0.0
                            } else {
                                c.bounds().width - s.bounds().width
                            },
                    ),
                y: c.visible_bounds()
                    .is_none_or(|vb| vb.height != c.bounds().height)
                    .then_some(
                        c.bounds().y - s.bounds().y, // + if c.bounds().y - s.bounds().y < translation.y {
                                                     // 0.0
                                                     // } else {
                                                     // c.bounds().height - s.bounds().height
                                                     // },
                    ),
            };

            Task::done(Message::ChatScrollToReplyFound(offset))
        })
}
