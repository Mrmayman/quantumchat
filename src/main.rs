/*
QuantumChat
Copyright (C) 2026 Mrmayman

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/

use sipper::sipper;
use std::{borrow::Cow, collections::HashMap, sync::Arc};
use tokio::{sync::Mutex, task::spawn_blocking};

use iced::Task;
use whatsmeow_nchat::{AccountState, ConnId};

use crate::{
    core::{App, IntoStringError, Message},
    state::{ChatUI, MenuChats, MenuLogin, State},
    storage::{DIR, Data},
    stylesheet::styles::{Theme, ThemeColor, ThemeMode},
    view::{chat::ID_MESSAGES, chat_buffer::ChatBuffer},
};

mod core;
mod events;
mod icons;
mod logic;
mod state;
mod storage;
#[allow(unused)]
mod stylesheet;
mod view;

pub const FONT_MONO: iced::Font = iced::Font::with_name("JetBrains Mono");
pub const FONT_DEFAULT: iced::Font = iced::Font::with_name("Inter");

type Element<'a> = iced::Element<'a, Message, Theme>;
type Res<T = ()> = Result<T, String>;

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
                message_drafts: HashMap::new(),
                state: State::Chats(MenuChats::new(), None),
                db,
                typing: HashMap::new(),
            },
            Task::perform(
                spawn_blocking(|| {
                    whatsmeow_nchat::create_connection(&*DIR, "", 1)
                        .strerr()
                        .map(|(a, b)| (a, Arc::new(Mutex::new(b))))
                }),
                |n| Message::Connected(n.strerr().and_then(|n| n)),
            ),
        )
    }

    pub fn update(&mut self, message: Message) -> Res<Task<Message>> {
        match message {
            Message::Nothing => {}
            Message::OpenMainMenu => {
                self.state = State::Chats(MenuChats::new(), None);
            }
            Message::Connected(r) => {
                let (id, recv) = r?;
                self.id = id;
                let acc = AccountState::get(id);
                println!("Account: {acc:?}");
                let login_t = if let AccountState::None = acc {
                    Task::perform(
                        spawn_blocking(move || whatsmeow_nchat::login(id).strerr()),
                        |_| Message::CoreTick,
                    )
                } else {
                    Task::perform(
                        spawn_blocking(move || whatsmeow_nchat::fetch_contacts(id).strerr()),
                        |n| Message::Done(n.strerr().and_then(|n| n)),
                    )
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

                return Ok(Task::batch([event_t, login_t]));
            }
            Message::Done(r) => r?,
            Message::CoreTick => return Ok(self.tick()),
            Message::CoreEvent(event, _status) => {
                if let iced::Event::Window(iced::window::Event::CloseRequested) = event {
                    whatsmeow_nchat::cleanup(self.id).unwrap();
                    std::process::exit(0);
                }
            }
            Message::SidebarResize(ratio) => {
                if let State::Chats(menu, _) = &mut self.state
                    && let Some(split) = menu.sidebar_split
                {
                    menu.sidebar_grid_state.resize(split, ratio);
                }
            }
            Message::ChatSelected(chat_id) => {
                if let State::Chats(_, ui) = &mut self.state {
                    let (chat_buffer, task) = ChatBuffer::new(&self.db, chat_id.clone())?;
                    *ui = Some(ChatUI {
                        selected: chat_id,
                        chat_buffer,
                        msg_hover: None,
                        animation_jump: None,
                    });
                    return Ok(Task::batch([
                        task,
                        iced::widget::operation::snap_to_end(ID_MESSAGES),
                    ]));
                }
            }
            Message::WEvent(event) => return self.handle_event(event),
            Message::ChatScrollLazyLoad(reverse) => {
                if let State::Chats(_, Some(chat)) = &mut self.state
                    && !chat.chat_buffer.debounce(reverse)
                {
                    return Ok(chat.chat_buffer.load_begin(&self.db, reverse));
                }
            }
            Message::ChatScrolledView(v) => {
                if let State::Chats(_, Some(chat)) = &mut self.state {
                    chat.chat_buffer.scroll = v;
                }
            }
            Message::ChatMessageInput(msg) => {
                if let State::Chats(_, Some(chat)) = &mut self.state {
                    let draft = self
                        .message_drafts
                        .entry(chat.selected.clone())
                        .or_default();
                    draft.text = msg;
                }
            }
            Message::ChatSend => {
                if let State::Chats(_, Some(chat)) = &mut self.state {
                    let chat_id = chat.selected.clone();
                    let Some(contents) = self.message_drafts.remove(&chat_id) else {
                        return Ok(Task::none());
                    };
                    if contents.text.is_empty() {
                        return Ok(Task::none());
                    }
                    let id = self.id;

                    return Ok(Task::perform(
                        spawn_blocking(move || {
                            let reply = contents.reply_to.map(|n| whatsmeow_nchat::QuotedMessage {
                                sender: n.sender,
                                contents: n.text,
                                message_id: n.id,
                            });
                            whatsmeow_nchat::send_message(
                                id,
                                &chat_id,
                                &contents.text,
                                reply.as_ref(),
                                None::<(&std::path::Path, _)>,
                                None,
                            )
                            .strerr()
                        }),
                        |n| Message::Done(n.strerr().and_then(|n| n)),
                    ));
                }
            }
            Message::ChatBufferLoaded(r) => {
                if let State::Chats(_, Some(chat)) = &mut self.state {
                    let r = r?;
                    let len = r.messages.len();
                    let reverse = r.is_reverse;
                    let viewport = chat.chat_buffer.scroll;

                    chat.chat_buffer.loaded(&self.db, r)?;

                    return Ok(if reverse {
                        let reverse_offset = viewport.absolute_offset_reversed();
                        iced::widget::operation::snap_to_end(ID_MESSAGES).chain(
                            iced::widget::operation::scroll_by(
                                ID_MESSAGES,
                                iced::widget::operation::AbsoluteOffset {
                                    x: -reverse_offset.x,
                                    y: -reverse_offset.y,
                                },
                            ),
                        )
                    } else {
                        iced::widget::operation::scroll_to(ID_MESSAGES, viewport.absolute_offset())
                    }
                    .chain(Task::perform(
                        async move {
                            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                            Message::ChatBufferShrink(len, reverse)
                        },
                        |n| n,
                    )));
                }
            }
            Message::ChatBufferShrink(len, reverse) => {
                if let State::Chats(_, Some(chat)) = &mut self.state {
                    chat.chat_buffer.shrink(len, reverse);
                    let viewport = chat.chat_buffer.scroll;

                    return Ok(if reverse {
                        iced::widget::operation::scroll_to(ID_MESSAGES, viewport.absolute_offset())
                    } else {
                        let reverse_offset = viewport.absolute_offset_reversed();
                        iced::widget::operation::snap_to_end(ID_MESSAGES).chain(
                            iced::widget::operation::scroll_by(
                                ID_MESSAGES,
                                iced::widget::operation::AbsoluteOffset {
                                    x: -reverse_offset.x,
                                    y: -reverse_offset.y,
                                },
                            ),
                        )
                    });
                }
            }

            Message::ChatScrollToReply(msg_id) => return Ok(self.scroll_to_reply(msg_id)),
            Message::ChatScrollToReplyFound(offset) => self.scroll_to_reply_done(offset),

            Message::ChatOpenProfile(jid) => {
                if let State::Chats(menu, _) = &mut self.state {
                    menu.opened_profile = jid;
                }
            }
            Message::ChatMsgHover(msg_id, entered) => {
                if let State::Chats(_, Some(chat)) = &mut self.state {
                    if entered {
                        chat.msg_hover = Some(msg_id);
                    } else if let Some(id) = &chat.msg_hover
                        && *id == msg_id
                    {
                        chat.msg_hover = None;
                    }
                }
            }
            Message::ChatReplyTo(msg_id) => {
                if let State::Chats(_, Some(chat)) = &mut self.state {
                    let draft = self
                        .message_drafts
                        .entry(chat.selected.clone())
                        .or_default();
                    draft.reply_to = msg_id;
                }
            }
        }
        Ok(Task::none())
    }

    fn tick(&mut self) -> Task<Message> {
        let mut task = Task::none();
        if let State::Chats(_, Some(chat)) = &mut self.state
            && let Some(animation) = &mut chat.animation_jump
        {
            let (t, finished) = animation.tick(chat.chat_buffer.scroll);
            if finished {
                chat.animation_jump = None;
            }
            task = t;
        }

        if self.db.contacts_sort_free {
            self.db.sort_contacts();
            self.db.contacts_sort_free = false;
        }
        if self.db.config_autosave_free {
            let contents =
                serde_json::to_string_pretty(&self.db.config).expect("should normally never fail");
            tokio::spawn(async move {
                let p = DIR.join("config.json");
                _ = tokio::fs::write(&p, contents).await;
            });
            self.db.config_autosave_free = false;
        }

        task
    }

    fn set_error(&mut self, err: String) {
        eprintln!("ERROR: {err}");
        self.state = State::Error(err);
    }

    fn go_to_login(&mut self, code: String, is_qr: bool) {
        self.state = match MenuLogin::new(code, is_qr) {
            Ok(menu) => State::Login(menu),
            Err(err) => State::Error(format!("While generating login QR:\n{err}")),
        };
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        let tick = iced::time::every(std::time::Duration::from_millis(1000 / 5))
            .map(|_| Message::CoreTick);
        let events = iced::event::listen_with(|a, b, _| Some(Message::CoreEvent(a, b)));

        let smooth_animation = if self.is_animating() {
            iced::window::frames().map(|_| Message::CoreTick)
        } else {
            iced::Subscription::none()
        };

        iced::Subscription::batch(vec![tick, events, smooth_animation])
    }

    #[must_use]
    fn is_animating(&self) -> bool {
        if let State::Chats(_, Some(ui)) = &self.state {
            ui.animation_jump.is_some()
        } else {
            false
        }
    }

    fn theme(&self) -> Theme {
        self.theme.clone()
    }
}

fn main() {
    const WINDOW_HEIGHT: f32 = 400.0;
    const WINDOW_WIDTH: f32 = 600.0;

    iced::application(
        App::create,
        |n: &mut App, m| match n.update(m) {
            Ok(n) => n,
            Err(err) => {
                n.set_error(err);
                Task::none()
            }
        },
        App::view,
    )
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
        exit_on_close_request: false,
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
