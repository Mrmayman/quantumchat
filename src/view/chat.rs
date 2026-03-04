use crate::{
    icons,
    state::{ChatUI, MenuChats},
    stylesheet::{
        color::Color,
        styles::{Theme, BORDER_RADIUS},
        widgets::StyleButton,
    },
    view::{
        chat_buffer::RenderedMessage,
        components::{button_with_icon, sbox, tsubtitle, underline_maybe},
    },
    App, Element, Message,
};

use iced::{
    widget::{self, column, row, text::Shaping},
    Alignment, Length,
};
use whatsmeow_nchat::Jid;

impl App {
    pub fn view_chats<'a>(&'a self, menu: &'a MenuChats, ui: Option<&'a ChatUI>) -> Element<'a> {
        widget::pane_grid(&menu.sidebar_grid_state, |_, is_sidebar, _| {
            if *is_sidebar {
                row![
                    sbox(self.view_chats_sidebar(ui), Color::ExtraDark),
                    widget::rule::vertical(1),
                ]
                .into()
            } else if let Some(ui) = ui {
                self.view_chats_page(ui).into()
            } else {
                sbox("Select a chat", Color::Dark)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(10)
                    .into()
            }
        })
        .on_resize(10, |t| Message::SidebarResize(t.ratio))
        .into()
    }

    fn view_chats_page<'a>(&'a self, ui: &'a ChatUI) -> widget::Container<'a, Message, Theme> {
        let sensor_up = (!ui.chat_buffer.messages.is_empty()).then_some(
            widget::sensor("...")
                .on_show(|_| Message::ChatScrollLazyLoad(true))
                .key(ui.chat_buffer.start_ts),
        );

        let sensor_down = self
            .db
            .contacts
            .get(&ui.chat_buffer.viewing)
            .filter(|n| n.last_message_time != ui.chat_buffer.end_ts)
            .map(|_| {
                widget::sensor("...")
                    .on_show(|_| Message::ChatScrollLazyLoad(false))
                    .key(ui.chat_buffer.end_ts)
            });

        widget::container(widget::column![
            sbox(
                widget::text(self.db.display_jid(&ui.selected))
                    .size(20)
                    .shaping(Shaping::Advanced),
                Color::Dark
            )
            .width(Length::Fill)
            .padding(16),
            widget::rule::horizontal(1),
            widget::scrollable(
                widget::Column::new()
                    .push(sensor_up)
                    .extend(ui.chat_buffer.messages.iter().map(|n| render_msg(n)))
                    .push(sensor_down)
                    .spacing(2)
                    .padding(10)
            )
            .id("messages")
            .style(|t: &Theme, s| t.style_scrollable_flat_dark(s))
            .width(Length::Fill)
            .height(Length::Fill)
            .on_scroll(Message::ChatScrolledView),
            widget::rule::horizontal(1),
            sbox(
                widget::row![
                    button_with_icon(icons::new_s(13), "", 13),
                    widget::text_input(
                        "Enter message...",
                        self.message_drafts
                            .get(&ui.selected)
                            .map(|n| n.as_str())
                            .unwrap_or_default()
                    )
                    .on_input(Message::ChatMessageInput)
                    .on_submit(Message::ChatSend),
                    button_with_icon(icons::checkmark_s(13), "Send", 13)
                        .on_press(Message::ChatSend)
                ]
                .spacing(5),
                Color::Dark
            )
            .padding(5),
        ])
        .style(move |t: &Theme| t.style_container_sharp_box(0.0, Color::ExtraDark))
        .width(Length::Fill)
        .height(Length::Fill)
    }

    fn view_chats_sidebar<'a>(&'a self, ui: Option<&'a ChatUI>) -> Element<'a> {
        column![
            row![icons::chatbox_s(20), widget::text("Chats").size(20)]
                .padding([10, 16])
                .spacing(10)
                .width(Length::Fill),
            widget::scrollable(widget::column(
                self.db
                    .config
                    .pins
                    .iter()
                    .chain(self.db.order.iter())
                    .map(|n| {
                        let Some(contact) = self.db.contacts.get(n) else {
                            return (
                                n,
                                row![
                                    widget::text("?").style(tsubtitle).size(14),
                                    widget::text(n.number()).style(tsubtitle)
                                ],
                            );
                        };

                        let indicators: Option<Element> = if let Some(typing) = self.typing.get(n) {
                            Some(
                                widget::text!("{} is typing...", self.db.display_jid(typing))
                                    .size(12)
                                    .style(tsubtitle)
                                    .into(),
                            )
                        } else if let (Some(sender), Some(line)) = (
                            contact
                                .last_msg_sender
                                .as_ref()
                                .and_then(|n| Jid::parse(n))
                                .map(|n| self.db.display_jid(&n).to_owned()),
                            contact
                                .last_msg_contents
                                .as_ref()
                                .and_then(|n| n.lines().next()),
                        ) {
                            fn t(t: widget::Text<Theme>) -> widget::Text<Theme> {
                                t.wrapping(widget::text::Wrapping::None)
                                    .shaping(Shaping::Advanced)
                                    .size(12)
                            }

                            Some(
                                row![
                                    t(widget::text!("{sender}: "))
                                        .style(|t: &Theme| t.style_text(Color::Mid)),
                                    t(widget::text(line)).style(tsubtitle)
                                ]
                                .into(),
                            )
                        } else {
                            None
                        };

                        (
                            n,
                            row![
                                icons::chatbox_s(14),
                                column![
                                    widget::text(&contact.name).shaping(Shaping::Advanced),
                                    indicators
                                ]
                                .spacing(2)
                            ],
                        )
                    })
                    .map(|(n, elem)| {
                        let is_selected = ui.as_ref().is_some_and(|ui| &ui.selected == n);
                        let button =
                            widget::button(elem.align_y(Alignment::Center).padding(5).spacing(10))
                                .on_press_maybe(
                                    (!is_selected).then_some(Message::ChatSelected(n.clone())),
                                )
                                .style(|n: &Theme, status| {
                                    n.style_button(status, StyleButton::FlatExtraDark)
                                })
                                .width(Length::Fill);

                        underline_maybe(button, Color::SecondDark, !is_selected)
                    })
            ))
            .height(Length::Fill)
            .width(Length::Fill)
            .style(|t: &Theme, s| t.style_scrollable_flat_extra_dark(s))
        ]
        .into()
    }
}

fn render_msg(msg: &RenderedMessage) -> Element<'_> {
    fn mbox<'a>(
        col: Color,
        e: impl Into<Element<'a>>,
        border: bool,
    ) -> widget::Container<'a, Message, Theme> {
        widget::container(e.into())
            .padding(5)
            .style(move |t: &Theme| widget::container::Style {
                border: {
                    iced::Border {
                        color: t.get(if border { col.next() } else { col }),
                        width: 1.0,
                        radius: BORDER_RADIUS.into(),
                    }
                },
                background: Some(t.get_bg(col)),
                ..Default::default()
            })
    }

    let time = || widget::text(&msg.time_display).size(10).style(tsubtitle);

    let edited = || {
        msg.is_edited
            .then_some(widget::text("(Edited)").size(10).style(tsubtitle))
    };

    let reply = msg.replying_to.as_ref().map(|reply| {
        mbox(
            if msg.from_me {
                Color::Dark
            } else {
                Color::SecondDark
            },
            column![
                widget::text(&reply.sender_name)
                    .size(12)
                    .shaping(Shaping::Advanced),
                widget::text(&reply.text).shaping(Shaping::Advanced),
            ],
            false,
        )
    });

    row![
        msg.from_me.then_some(widget::space::horizontal()),
        column![mbox(
            if msg.from_me {
                Color::SecondDark
            } else {
                Color::Dark
            },
            column![
                reply,
                column![
                    (!msg.from_me).then_some(
                        row![
                            widget::text(&msg.message.sender_name)
                                .size(12)
                                .shaping(Shaping::Advanced),
                            edited(),
                            time()
                        ]
                        .spacing(10)
                    ),
                    widget::text(&msg.message.text),
                    msg.from_me.then_some(row![edited(), time()].spacing(10))
                ]
            ]
            .spacing(5),
            !msg.from_me,
        )]
        .extend(msg.reactions.iter().map(|n| {
            row![
                msg.from_me.then_some(widget::space::horizontal()),
                widget::text(if n.from_me { "(Me)" } else { &n.sender_name })
                    .size(10)
                    .style(tsubtitle),
                widget::text(&n.emoji).size(14),
            ]
            .align_y(Alignment::Center)
            .spacing(5)
            .into()
        }))
        .align_x(if msg.from_me {
            Alignment::End
        } else {
            Alignment::Start
        })
    ]
    .into()
}
