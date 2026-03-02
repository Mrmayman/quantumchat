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

impl App {
    pub fn view_chats<'a>(&'a self, menu: &'a MenuChats, ui: Option<&'a ChatUI>) -> Element<'a> {
        widget::pane_grid(&menu.sidebar_grid_state, |_, is_sidebar, _| {
            if *is_sidebar {
                row![
                    sbox(self.view_chats_sidebar(ui), Color::ExtraDark),
                    widget::rule::vertical(1),
                ]
                .into()
            } else {
                if let Some(ui) = ui {
                    self.view_chats_page(ui).into()
                } else {
                    sbox("Select a chat", Color::Dark)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .padding(10)
                        .into()
                }
            }
        })
        .on_resize(10, |t| Message::SidebarResize(t.ratio))
        .into()
    }

    fn view_chats_page<'a>(&'a self, ui: &'a ChatUI) -> widget::Container<'a, Message, Theme> {
        {
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
                        .push(widget::sensor("...").on_show(|_| Message::ChatScrolled(true)))
                        .extend(ui.chat_buffer.messages.iter().map(|n| render_msg(n)))
                        .push(
                            self.db
                                .contacts
                                .get(&ui.chat_buffer.viewing)
                                .filter(|n| n.last_message_time != ui.chat_buffer.end_ts)
                                .map(|_| {
                                    widget::sensor("...").on_show(|_| Message::ChatScrolled(false))
                                })
                        )
                        .spacing(2)
                        .padding(10)
                )
                .style(|t: &Theme, s| t.style_scrollable_flat_dark(s))
                .width(Length::Fill)
                .height(Length::Fill),
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
        }
        .width(Length::Fill)
        .height(Length::Fill)
    }

    fn view_chats_sidebar<'a>(&'a self, ui: Option<&'a ChatUI>) -> Element<'a> {
        column![
            row![icons::chatbox_s(20), widget::text("Chats").size(20)]
                .padding([10, 16])
                .spacing(10),
            widget::scrollable(widget::column(
                self.db
                    .config
                    .pins
                    .iter()
                    .chain(self.db.order.iter())
                    .map(|n| {
                        let Some(contact) = self.db.contacts.get(&n) else {
                            return (
                                n,
                                row![
                                    widget::text("?").style(tsubtitle).size(14),
                                    widget::text(n.number()).style(tsubtitle)
                                ],
                            );
                        };

                        let indicators = row![
                            if let Some(typing) = self.typing.get(n) {
                                Some(
                                    widget::text!("{} is typing...", self.db.display_jid(typing))
                                        .size(12)
                                        .style(tsubtitle),
                                )
                            } else if let Some(line) =
                                contact.last_msg.as_ref().and_then(|n| n.1.lines().next())
                            {
                                Some(
                                    widget::text(line)
                                        .wrapping(widget::text::Wrapping::None)
                                        .shaping(Shaping::Advanced)
                                        .size(12)
                                        .style(tsubtitle),
                                )
                            } else {
                                None
                            },
                            widget::space::horizontal(),
                            if let Some((_, _, time)) = &contact.last_msg.as_ref() {
                                Some(widget::text(time).size(12).style(tsubtitle))
                            } else {
                                None
                            },
                        ];

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
        if msg.from_me {
            Some(widget::space::horizontal())
        } else {
            None
        },
        mbox(
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
        ),
    ]
    .into()
}
