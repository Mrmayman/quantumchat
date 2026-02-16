use sipper::sipper;
use std::{borrow::Cow, sync::Arc};
use tokio::{
    sync::{
        mpsc::{self, UnboundedReceiver as Receiver},
        Mutex,
    },
    task::spawn_blocking,
};

use iced::Task;
use whatsmeow_nchat::{AccountState, ChatEvent, ConnId, Jid};

use crate::{
    state::{ChatUI, MenuChats, MenuLogin, State},
    storage::{contact::Contact, Data, DIR},
    stylesheet::styles::{Theme, ThemeColor, ThemeMode},
};

mod core;
mod icons;
mod state;
mod storage;
#[allow(unused)]
mod stylesheet;
mod view;

pub const FONT_MONO: iced::Font = iced::Font::with_name("JetBrains Mono");
pub const FONT_DEFAULT: iced::Font = iced::Font::with_name("Inter");

type Element<'a> = iced::Element<'a, Message, Theme>;
type WEvent = whatsmeow_nchat::Event;
type Res<T = ()> = Result<T, String>;

#[derive(Debug, Clone)]
enum Message {
    Nothing,
    Connected(Res<(ConnId, Arc<Mutex<Receiver<WEvent>>>)>),
    CoreTick,
    WEvent(WEvent),
    LoggedIn(Res),
    CoreEvent(iced::event::Event, iced::event::Status),
    SidebarResize(f32),
    ChatSelected(Jid),
}

struct App {
    id: ConnId,
    theme: Theme,
    state: State,
    db: Data,
}

impl App {
    pub fn create() -> (Self, Task<Message>) {
        println!("Starting up");

        let db = Data::new().unwrap();

        (
            Self {
                id: ConnId::from_inner(0),
                theme: Theme {
                    mode: ThemeMode::Dark,
                    color: ThemeColor::Purple,
                    alpha: 1.0,
                    system_dark_mode: true,
                },
                state: State::Chats(MenuChats::new(), None),
                db,
            },
            Task::perform(
                spawn_blocking(|| {
                    whatsmeow_nchat::create_connection(&*DIR, "", 1)
                        .strerr()
                        .map(|(a, b)| (a, Arc::new(Mutex::new(b))))
                }),
                |n| Message::Connected(n.strerr().flatten()),
            ),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Nothing => {}
            Message::Connected(r) => match r {
                Ok((id, recv)) => {
                    self.id = id;
                    let acc = AccountState::get(id);
                    println!("Account: {acc:?}");
                    let login_t = if let AccountState::None = acc {
                        Task::perform(
                            spawn_blocking(move || whatsmeow_nchat::login(id).strerr()),
                            |_| Message::CoreTick,
                        )
                    } else {
                        Task::none()
                    };

                    let event_t = Task::sip(
                        sipper(|mut sender| async move {
                            let mut r = recv.lock().await;
                            while let Some(evt) = r.recv().await {
                                sender.send(evt).await;
                            }
                        }),
                        Message::WEvent,
                        |()| Message::Nothing,
                    );

                    return Task::batch([event_t, login_t]);
                }
                Err(err) => self.set_error(err),
            },
            Message::LoggedIn(r) => {
                if let Err(err) = r {
                    self.set_error(err);
                }
            }
            Message::CoreTick => {}
            Message::CoreEvent(_event, _status) => {}
            Message::SidebarResize(ratio) => {
                if let State::Chats(menu, _) = &mut self.state {
                    if let Some(split) = menu.sidebar_split {
                        menu.sidebar_grid_state.resize(split, ratio);
                    }
                }
            }
            Message::ChatSelected(chat_id) => {
                if let State::Chats(_, ui) = &mut self.state {
                    *ui = Some(ChatUI { selected: chat_id });
                }
            }
            Message::WEvent(event) => match self.handle_event(event) {
                Ok(n) => return n,
                Err(err) => self.set_error(err),
            },
        }
        Task::none()
    }

    fn set_error(&mut self, err: String) {
        eprintln!("ERROR: {err}");
        self.state = State::Error(err);
    }

    fn handle_event(&mut self, event: WEvent) -> Res<Task<Message>> {
        if let State::Login(_) = &self.state {
            self.state = State::Chats(MenuChats::new(), None);
            return Ok(Task::none());
        }
        let (id, event) = match event {
            WEvent::ChatEvent(jid, chat_event) => (jid, chat_event),
            WEvent::QrCode(code) => {
                self.go_to_login(code, true);
                return Ok(Task::none());
            }
            WEvent::PairingCode(code) => {
                self.go_to_login(code, false);
                return Ok(Task::none());
            }
            WEvent::Reinit => {
                let id = self.id;
                self.state = State::Loading;
                return Ok(Task::perform(
                    tokio::task::spawn_blocking(move || {
                        whatsmeow_nchat::logout(id).strerr()?;
                        whatsmeow_nchat::login(id).strerr()?;
                        Ok::<(), String>(())
                    }),
                    |n| Message::LoggedIn(n.strerr().flatten()),
                ));
            }
            _ => {
                let message = format!("{event:?}")
                    .split(',')
                    .filter(|n| n.contains(['}', '{', '(', ')', '[', ']']) || !n.contains(": None"))
                    .collect::<String>();
                println!("WEVENT {message}\n");
                return Ok(Task::none());
            }
        };

        match event {
            ChatEvent::NewContactsNotify {
                name,
                phone,
                is_self,
                is_group,
                notify,
            } => self.db.add_contact(Contact {
                name,
                jid: Jid::from_phone_no(&phone),
                muted: false, // TODO
                is_group,
            })?,
            ChatEvent::NewChatsNotify {
                is_unread,
                is_muted,
                is_pinned,
                last_message_time,
            } => println!("CHAT {id:?}: {last_message_time}"),
            _ => {
                let message = format!("{event:?}")
                    .split(',')
                    .filter(|n| n.contains(['}', '{', '(', ')', '[', ']']) || !n.contains(": None"))
                    .collect::<String>();
                println!("WEVENT {id:?} {message}\n");
            }
        }
        Ok(Task::none())
    }

    fn go_to_login(&mut self, code: String, is_qr: bool) {
        self.state = match MenuLogin::new(code.clone(), is_qr) {
            Ok(menu) => State::Login(menu),
            Err(err) => State::Error(format!("While generating login QR:\n{err}")),
        };
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

    iced::application(App::create, App::update, App::view)
        .title(|_: &App| "QuantumChat".to_owned())
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
        .run()
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

pub trait IntoStringError<T> {
    #[allow(clippy::missing_errors_doc)]
    fn strerr(self) -> Result<T, String>;
}

impl<T, E: ToString> IntoStringError<T> for Result<T, E> {
    fn strerr(self) -> Result<T, String> {
        self.map_err(|err| err.to_string())
    }
}
