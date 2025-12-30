use std::borrow::Cow;

use iced::{Length, Task, widget};

use crate::stylesheet::{
    color::Color,
    styles::{LauncherThemeColor, LauncherThemeLightness, Theme},
};

#[allow(unused)]
mod stylesheet;

pub const FONT_MONO: iced::Font = iced::Font::with_name("JetBrains Mono");
pub const FONT_DEFAULT: iced::Font = iced::Font::with_name("Inter");

type Element<'a> = iced::Element<'a, Message, Theme>;

#[derive(Debug, Clone)]
enum Message {
    CoreTick,
    CoreEvent(iced::event::Event, iced::event::Status),
}

struct App {
    theme: Theme,
}

impl App {
    pub fn create() -> (Self, Task<Message>) {
        (
            Self {
                theme: Theme {
                    lightness: LauncherThemeLightness::Dark,
                    color: LauncherThemeColor::Purple,
                    alpha: 1.0,
                    system_dark_mode: true,
                },
            },
            Task::none(),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::CoreTick => {}
            Message::CoreEvent(_event, _status) => {}
        }
        Task::none()
    }

    pub fn view(&self) -> Element<'_> {
        widget::container(widget::column!["hello there!"].padding(10))
            .style(|t: &Theme| t.style_container_sharp_box(0.0, Color::Dark))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    #[allow(clippy::unused_self)]
    fn subscription(&self) -> iced::Subscription<Message> {
        let tick = iced::time::every(std::time::Duration::from_millis(1000 / 5))
            .map(|_| Message::CoreTick);
        let events = iced::event::listen_with(|a, b, _| Some(Message::CoreEvent(a, b)));

        iced::Subscription::batch(vec![tick, events])
    }

    fn theme(&self) -> Theme {
        self.theme.clone()
    }
}

fn main() {
    const WINDOW_HEIGHT: f32 = 400.0;
    const WINDOW_WIDTH: f32 = 600.0;

    iced::application("QuantumChat", App::update, App::view)
        .subscription(App::subscription)
        // .scale_factor(App::scale_factor)
        .theme(App::theme)
        .settings(iced::Settings {
            fonts: load_fonts(),
            default_font: FONT_DEFAULT,
            // antialiasing: true,
            ..Default::default()
        })
        .window(iced::window::Settings {
            // icon,
            // exit_on_close_request: false,
            size: iced::Size {
                width: WINDOW_WIDTH,
                height: WINDOW_HEIGHT,
            },
            min_size: Some(iced::Size {
                width: 420.0,
                height: 310.0,
            }),
            // decorations,
            // transparent: true,
            ..Default::default()
        })
        .run_with(move || App::create())
        .unwrap();
}

fn load_fonts() -> Vec<Cow<'static, [u8]>> {
    vec![
        include_bytes!("../assets/fonts/Inter-Regular.ttf")
            .as_slice()
            .into(),
        include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf")
            .as_slice()
            .into(),
        include_bytes!("../assets/fonts/password_asterisks/password-asterisks.ttf")
            .as_slice()
            .into(),
        include_bytes!("../assets/fonts/icons.ttf")
            .as_slice()
            .into(),
    ]
}
