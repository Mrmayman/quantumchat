use iced::{
    Length,
    widget::{self, column, row},
};

use crate::{
    App, Element, FONT_MONO, Message,
    state::{MenuLogin, State},
    stylesheet::color::Color,
    view::components::{center, sbox},
};

pub mod chat;
pub mod chat_buffer;
mod components;
mod rich_text;

impl App {
    pub fn view(&self) -> Element<'_> {
        let view: Element = match &self.state {
            State::Loading => center("Loading...").into(),
            State::Login(menu) => menu.view(),
            State::Chats(menu, ui) => self.view_chats(menu, ui.as_ref()),
            State::Error(err) => center(column![
                widget::text("Error").size(20),
                widget::text(err).size(14),
                widget::button("Back").on_press(Message::OpenMainMenu)
            ])
            .into(),
        };
        sbox(view, Color::Dark)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

// TODO: Auto scroll down and refresh buffer when new msg come

impl MenuLogin {
    pub fn view(&self) -> Element<'_> {
        let code: Element = if let Some(qr) = &self.qr_code {
            widget::qr_code(qr).cell_size(2).into()
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
                    widget::button(widget::text("View Existing Data...").size(14)).on_press(Message::OpenMainMenu)
                ]
                .spacing(2),
                code
            ]
            .spacing(16),
        )
        .into()
    }
}
