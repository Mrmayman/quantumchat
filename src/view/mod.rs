use iced::{Length, widget};
use widget::{column, row};

use crate::{
    App, Element,
    state::{MenuLogin, State},
    stylesheet::{color::Color, styles::Theme},
    view::components::center,
};

mod components;

impl App {
    pub fn view(&self) -> Element<'_> {
        let view: Element = match &self.state {
            State::Loading => center("Loading...").into(),
            State::Login(menu) => menu.view(),
            State::Chats => center("TODO: Add chat functionality").into(),
            State::Error(err) => center(column![
                widget::text("Error").size(20),
                widget::text(err).size(14),
            ])
            .into(),
        };
        widget::container(view)
            .style(|t: &Theme| t.style_container_sharp_box(0.0, Color::Dark))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl MenuLogin {
    pub fn view(&self) -> Element<'_> {
        let elapsed = self.initial_time.elapsed();
        let qr: Element = if elapsed < self.timeout {
            let time_left = self.timeout - elapsed;
            column![
                widget::qr_code(&self.qr_code).cell_size(2),
                widget::text!("Scan in {}s", time_left.as_secs())
            ]
            .spacing(10)
            .into()
        } else {
            widget::text("QR code expired! Clone and reopen this app").into()
        };

        center(
            row![
                column![
                    widget::text("Steps to log in"),
                    widget::text("1. Open WhatsApp on your phone").size(12),
                    widget::text("2. On Android tap menu (...), on iPhone tap Settings").size(12),
                    widget::text("3. Tap Linked Devices, then Link Device").size(12),
                    widget::text("4. Scan the QR code and wait").size(12),
                ]
                .spacing(2),
                qr
            ]
            .spacing(16),
        )
        .into()
    }
}
