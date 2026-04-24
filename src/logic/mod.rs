use iced::{
    Task,
    advanced::graphics::futures::MaybeSend,
    widget::{self, operation, selector},
};
use whatsmeow_nchat::MsgId;

use crate::core::{App, Message};

impl App {
    pub(super) fn scroll_to_reply(&mut self, msg_id: MsgId) -> Task<Message> {
        let widget_id = format!("msg:{}", msg_id.0);
        self.animations.new_reply(msg_id);
        scroll_into_view("messages", widget_id)
    }
}

// Adapted from generic-daw:
// https://github.com/generic-daw/generic-daw/blob/main/generic_daw_gui/src/operation.rs
//
// Copyright (C) 2026 edwloef
// Licensed under the GNU General Public License v3.0
pub fn scroll_into_view<T: MaybeSend + 'static>(
    scrollable: impl Into<widget::Id>,
    child: impl Into<widget::Id>,
) -> Task<T> {
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
                        c.bounds().y - s.bounds().y
                            + if c.bounds().y - s.bounds().y < translation.y {
                                0.0
                            } else {
                                c.bounds().height - s.bounds().height
                            },
                    ),
            };

            operation::scroll_to(scrollable.clone(), offset)
        })
}
