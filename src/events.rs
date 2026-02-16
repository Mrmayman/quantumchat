use iced::Task;
use whatsmeow_nchat::{ChatEvent, Event, Jid};

use crate::{
    state::{MenuChats, State},
    storage::{contact::Contact, message::MsgData},
    App, IntoStringError, Message,
};

impl App {
    pub fn handle_event(&mut self, event: Event) -> Result<Task<Message>, String> {
        if let State::Login(_) = &self.state {
            self.state = State::Chats(MenuChats::new(), None);
            return Ok(Task::none());
        }
        let (id, event) = match event {
            Event::ChatEvent(jid, chat_event) => (jid, chat_event),
            Event::QrCode(code) => {
                self.go_to_login(code, true);
                return Ok(Task::none());
            }
            Event::PairingCode(code) => {
                self.go_to_login(code, false);
                return Ok(Task::none());
            }
            Event::Reinit => return self.e_reinit(),
            _ => {
                dbg_print("GLOBALEVENT", &event);
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
            } => {
                let jid = if phone.is_empty() {
                    id
                } else {
                    Jid::from_phone_no(phone)
                };
                if is_self {
                    self.db.config.self_jid = Some(jid.clone());
                    self.db.save_config()?;
                }
                self.db.add_contact(Contact {
                    name,
                    jid,
                    muted: false,
                    is_group,
                    chatted: false,
                    last_message_time: 0,
                    last_read_message_time: 0,
                })?;
            }
            ChatEvent::NewChatsNotify {
                is_unread,
                is_muted,
                is_pinned,
                last_message_time,
            } => {
                self.db.operate_on_contact(&id, |n| {
                    n.chatted = true;
                    n.muted = is_muted;
                    let time = last_message_time as i64;
                    if n.last_message_time < time {
                        n.last_message_time = time;
                    }
                    if n.last_read_message_time < time && !is_unread {
                        n.last_read_message_time = time;
                    }
                })?;
                self.db.sort_contacts();
                println!("CHAT {id:?}: {last_message_time}");
                self.db.add_pin(id, is_pinned)?;
            }
            ChatEvent::NewMessagesNotify {
                msg_id,
                sender_id,
                text,
                from_me,
                quoted_id,
                file_id_path,
                file_status,
                time_sent,
                is_read,
                is_edited,
            } => self.db.add_message(MsgData {
                c: text,
                sender: (sender_id != id).then_some(sender_id),
                src: id,
                replying_to: quoted_id,
                timestamp: time_sent as i64,
                msg_id,
                is_edited,
                is_read,
                from_me,
            })?,
            _ => {
                let message = format!("{event:?}")
                    .split(',')
                    .filter(|n| n.contains(['}', '{', '(', ')', '[', ']']) || !n.contains(": None"))
                    .collect::<String>();
                println!("Event {id:?} {message}\n");
            }
        }
        Ok(Task::none())
    }

    fn e_reinit(&mut self) -> Result<Task<Message>, String> {
        let id = self.id;
        self.state = State::Loading;
        Ok(Task::perform(
            tokio::task::spawn_blocking(move || {
                whatsmeow_nchat::logout(id).strerr()?;
                whatsmeow_nchat::login(id).strerr()?;
                Ok::<(), String>(())
            }),
            |n| Message::LoggedIn(n.strerr().flatten()),
        ))
    }
}

fn dbg_print<T: std::fmt::Debug>(pref: &str, thing: &T) {
    let message = format!("{thing:?}")
        .split(',')
        .filter(|n| n.contains(['}', '{', '(', ')', '[', ']']) || !n.contains(": None"))
        .collect::<String>();
    println!("{pref} {message}\n");
}
