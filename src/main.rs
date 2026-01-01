use std::{
    borrow::Cow,
    sync::mpsc::{self, Receiver},
};

use iced::Task;

use crate::{
    state::{MenuLogin, State},
    storage::{Data, contact::Jid},
    stylesheet::styles::{LauncherThemeColor, LauncherThemeLightness, Theme},
};

mod core;
mod init;
mod state;
mod storage;
#[allow(unused)]
mod stylesheet;
mod view;

pub const FONT_MONO: iced::Font = iced::Font::with_name("JetBrains Mono");
pub const FONT_DEFAULT: iced::Font = iced::Font::with_name("Inter");

type Element<'a> = iced::Element<'a, Message, Theme>;
type WEvent = whatsapp_rust::types::events::Event;

#[derive(Debug, Clone)]
enum Message {
    CoreTick,
    CoreEvent(iced::event::Event, iced::event::Status),
}

struct App {
    theme: Theme,
    event_recv: Receiver<WEvent>,
    state: State,
    db: Data,
}

impl App {
    pub fn create() -> (Self, Task<Message>) {
        let (sender, receiver) = mpsc::channel();
        println!("Starting up");

        let db = Data::new().unwrap();

        (
            Self {
                theme: Theme {
                    lightness: LauncherThemeLightness::Dark,
                    color: LauncherThemeColor::Purple,
                    alpha: 1.0,
                    system_dark_mode: true,
                },
                event_recv: receiver,
                state: State::Loading,
                db,
            },
            Task::perform(init::init(sender), |()| Message::CoreTick),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::CoreTick => {
                while let Ok(event) = self.event_recv.try_recv() {
                    self.handle_event(event);
                }
            }
            Message::CoreEvent(_event, _status) => {}
        }
        Task::none()
    }

    fn handle_event(&mut self, event: WEvent) {
        if let WEvent::PairingQrCode { code, timeout } = &event {
            self.state = match MenuLogin::new(code.clone(), timeout.clone()) {
                Ok(menu) => State::Login(menu),
                Err(err) => State::Error(format!("While generating login QR:\n{err}")),
            };
            return;
        }
        if let State::Loading | State::Login(_) = &self.state {
            self.state = State::Chats;
            return;
        }

        match event {
            WEvent::ContactUpdate(contact) => att(self.db.add_contact(contact)),
            WEvent::MuteUpdate(mute) => att(self.db.add_mute(
                Jid {
                    user: mute.jid.user,
                    server: mute.jid.server,
                },
                mute.action.muted(),
                // TODO: mute expiry date
            )),
            WEvent::JoinedGroup(_) => {}
            _ => {
                let message = format!("{event:?}")
                    .split(',')
                    .filter(|n| n.contains(['}', '{', '(', ')', '[', ']']) || !n.contains(": None"))
                    .collect::<String>();
                println!("{message}\n");
            }
        }
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

fn att<T>(r: Result<T, String>) {
    if let Err(e) = r {
        println!("Error: {}", e);
    }
}
