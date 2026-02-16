use iced::{widget, Alignment, Length};
use whatsmeow_nchat::Jid;
use widget::{column, row};

use crate::{
    icons,
    state::{ChatUI, MenuChats, MenuLogin, State},
    stylesheet::{color::Color, styles::Theme, widgets::StyleButton},
    view::components::{button_with_icon, center, sbox, tsubtitle, underline_maybe},
    App, Element, Message, FONT_MONO,
};

mod components;

impl App {
    pub fn view(&self) -> Element<'_> {
        let view: Element = match &self.state {
            State::Loading => center("Loading...").into(),
            State::Login(menu) => menu.view(),
            State::Chats(menu, ui) => self.view_chats(menu, ui.as_ref()),
            State::Error(err) => center(column![
                widget::text("Error").size(20),
                widget::text(err).size(14),
            ])
            .into(),
        };
        sbox(view, Color::Dark)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    pub fn view_chats<'a>(&'a self, menu: &'a MenuChats, ui: Option<&'a ChatUI>) -> Element<'a> {
        widget::pane_grid(&menu.sidebar_grid_state, |_, is_sidebar, _| {
            if *is_sidebar {
                sbox(self.view_chats_sidebar(ui), Color::ExtraDark).into()
            } else {
                self.view_chats_page(ui).into()
            }
        })
        .on_resize(10, |t| Message::SidebarResize(t.ratio))
        .into()
    }

    fn view_chats_page<'a>(
        &'a self,
        ui: Option<&'a ChatUI>,
    ) -> widget::Container<'a, Message, Theme> {
        sbox(
            if let Some(ui) = ui {
                widget::column![
                    sbox(
                        widget::text(self.render_jid(&ui.selected)).size(20),
                        Color::Dark
                    )
                    .padding(16),
                    widget::rule::horizontal(1),
                    widget::scrollable(widget::column!["TODO: Implement chat"].padding(10))
                        .style(|t: &Theme, s| t.style_scrollable_flat_dark(s))
                        .width(Length::Fill)
                        .height(Length::Fill),
                    widget::rule::horizontal(1),
                    sbox(
                        widget::row![
                            button_with_icon(icons::new_s(13), "", 13),
                            widget::text_input("Enter message...", ""),
                            button_with_icon(icons::checkmark_s(13), "Send", 13)
                        ]
                        .spacing(5),
                        Color::Dark
                    )
                    .padding(5),
                ]
            } else {
                widget::column!["Select a chat"].padding(10)
            },
            Color::Dark,
        )
        .width(Length::Fill)
        .height(Length::Fill)
    }

    fn view_chats_sidebar<'a>(&'a self, ui: Option<&'a ChatUI>) -> Element<'a> {
        column![
            row![icons::chatbox_s(20), widget::text("Chats").size(20)]
                .padding(10)
                .spacing(10),
            widget::scrollable(widget::column(
                self.db
                    .config
                    .pins
                    .iter()
                    .chain(self.db.order.iter())
                    .map(|n| {
                        let Some(contact) = self.db.contacts.get(&n.to_id()) else {
                            return (
                                n,
                                widget::row![
                                    widget::text("?").style(tsubtitle).size(14),
                                    widget::text(n.number()).style(tsubtitle)
                                ],
                            );
                        };

                        (
                            n,
                            widget::row![icons::chatbox_s(14), widget::text(&contact.name)],
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

    pub fn render_jid<'a>(&'a self, jid: &'a Jid) -> &'a str {
        self.db
            .contacts
            .get(&jid.to_id())
            .map_or(jid.number(), |n| &n.name)
    }
}

impl MenuLogin {
    pub fn view(&self) -> Element<'_> {
        let code: Element = if let Some(qr) = &self.qr_code {
            widget::qr_code(&qr).cell_size(2).into()
        } else {
            widget::container(column![
                widget::text(&self.code).font(FONT_MONO).size(20),
                widget::button("Copy")
            ])
            .padding(16)
            .into()
        };
        center(
            row![
                column![
                    widget::text("Steps to log in"),
                    widget::text("1. Open WhatsApp on your phone").size(12),
                    widget::text("2. On Android tap menu (...), on iPhone tap Settings").size(12),
                    widget::text("3. Tap Linked Devices, then Link Device").size(12),
                    widget::text(if self.qr_code.is_some() {
                        "4. Scan the QR code and wait"
                    } else {
                        "4. Tap \"Link with phone number instead\" and enter this code on your phone"
                    }).size(12),
                ]
                .spacing(2),
                code
            ]
            .spacing(16),
        )
        .into()
    }
}
