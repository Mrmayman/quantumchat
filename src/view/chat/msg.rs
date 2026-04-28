use iced::{
    Alignment, Length,
    widget::{self, column, row, text::Shaping},
};
use whatsmeow_nchat::Jid;

use crate::{
    Element, FONT_EMOJI,
    core::{App, Message},
    state::ChatUI,
    stylesheet::{
        color::Color,
        styles::{BORDER_RADIUS, Theme},
        widgets::StyleButton,
    },
    view::{chat_buffer::RenderedMessage, components::tsubtitle},
};

impl App {
    pub(super) fn view_msg<'a>(ui: &ChatUI, msg: &'a RenderedMessage) -> Element<'a> {
        let show_top_bar = !msg.from_me && !msg.hide_sender;
        let is_hovered = ui.msg_hover.as_ref().is_some_and(|n| *n == msg.message.id);

        let hover_ctrl = || {
            widget::button(widget::text("←").size(14))
                .padding(8)
                .style(|t: &Theme, s| t.style_button(s, StyleButton::FlatDark))
                .on_press_with(|| Message::ChatReplyTo(Some(msg.message.clone())))
        };

        widget::mouse_area(
            widget::container(
                row![
                    msg.from_me.then_some(widget::space::horizontal()),
                    (msg.from_me && is_hovered).then(hover_ctrl),
                    column![
                        show_top_bar.then_some(widget::space().height(5)),
                        Self::view_msg_box(ui, msg, show_top_bar)
                    ]
                    .extend(reactions(msg))
                    .align_x(if msg.from_me {
                        Alignment::End
                    } else {
                        Alignment::Start
                    }),
                    (!msg.from_me && is_hovered).then(hover_ctrl),
                ]
                .align_y(Alignment::Center)
                .width(Length::Fill),
            )
            .style(move |t: &Theme| widget::container::Style {
                background: is_hovered.then_some(t.get_bg(Color::Dark)),
                ..Default::default()
            }),
        )
        .on_enter(Message::ChatMsgHover(msg.message.id.clone(), true))
        .on_exit(Message::ChatMsgHover(msg.message.id.clone(), false))
        .into()
    }

    fn view_msg_box<'a>(
        ui: &ChatUI,
        msg: &'a RenderedMessage,
        show_top_bar: bool,
    ) -> widget::Container<'a, Message, Theme> {
        let col = if msg.from_me {
            Color::SecondDark
        } else {
            Color::Dark
        };

        let content = column![
            view_reply(msg),
            show_top_bar.then(|| msg_header(msg)),
            widget::rich_text(&msg.message.text),
            (!show_top_bar).then(|| msg_footer(msg))
        ]
        .align_x(if msg.from_me {
            Alignment::End
        } else {
            Alignment::Start
        });

        let (border, width) = Self::view_msg_border(ui, msg, col);

        widget::container(content)
            .padding(5)
            .style(move |t: &Theme| widget::container::Style {
                border: {
                    iced::Border {
                        color: t.get(border),
                        width,
                        radius: BORDER_RADIUS.into(),
                    }
                },
                background: Some(t.get_bg(col)),
                ..Default::default()
            })
            .id(format!("msg:{}", msg.message.id.0))
    }

    fn view_msg_border(ui: &ChatUI, msg: &RenderedMessage, col: Color) -> (Color, f32) {
        if let Some(anim) = &ui.animation_jump
            && anim.to_msg == msg.message.id
        {
            (col.next().next(), 4.0)
        } else if msg.from_me {
            (col, 0.0)
        } else {
            (col.next(), 1.0)
        }
    }
}

fn reactions(msg: &RenderedMessage) -> impl Iterator<Item = Element<'_>> + '_ {
    msg.reactions.iter().map(|n| {
        row![
            msg.from_me.then_some(widget::space::horizontal()),
            sender_link(
                if n.from_me { "(Me)" } else { &n.sender_name },
                n.sender.clone()
            )
            .size(10),
            widget::text(&n.emoji)
                .size(14)
                .shaping(Shaping::Advanced)
                .font(FONT_EMOJI)
        ]
        .align_y(Alignment::Center)
        .spacing(5)
        .padding(iced::Padding::ZERO.left(10))
        .into()
    })
}

fn edited(msg: &RenderedMessage) -> Option<widget::Text<'_, Theme>> {
    msg.is_edited
        .then_some(widget::text("(Edited)").size(10).style(tsubtitle))
}

fn msg_footer(msg: &RenderedMessage) -> widget::Row<'_, Message, Theme> {
    row![edited(msg), time(msg)]
        .align_y(Alignment::Center)
        .spacing(10)
}

pub fn sender_link<'a>(name: &'a str, jid: Jid) -> widget::text::Rich<'a, Jid, Message, Theme> {
    widget::rich_text![widget::span(name).link(jid)]
        .on_link_click(|n: Jid| Message::ChatOpenProfile(Some(n)))
        .size(12)
}

fn msg_header<'a>(msg: &'a RenderedMessage) -> widget::Row<'a, Message, Theme> {
    row![
        sender_link(&msg.message.sender_name, msg.message.sender.clone()),
        time(msg),
        edited(msg),
    ]
    .align_y(Alignment::Center)
    .spacing(10)
}

fn time(msg: &RenderedMessage) -> widget::Text<'_, Theme> {
    widget::text(&msg.time_display).size(10).style(tsubtitle)
}

fn view_reply(msg: &RenderedMessage) -> Option<widget::Column<'_, Message, Theme>> {
    msg.replying_to.as_ref().map(|reply| {
        column![
            widget::button(column![
                // sender_link(&msg.message.sender_name, msg.message.sender.clone()),
                widget::text(&reply.sender_name).size(12).style(tsubtitle),
                widget::rich_text(&reply.text)
            ])
            .style(|t: &Theme, s| t.style_button(
                s,
                if msg.from_me {
                    StyleButton::RoundDark
                } else {
                    StyleButton::Round
                }
            ))
            .on_press(Message::ChatScrollToReply(reply.id.clone())),
            widget::space().height(5),
        ]
    })
}
