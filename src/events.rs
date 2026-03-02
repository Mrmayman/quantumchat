use iced::Task;
use whatsmeow_nchat::{ChatEvent, Event, Jid};

use crate::{
    state::{MenuChats, State},
    storage::{contact::Contact, message::MsgData, Time},
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
                    id.clone()
                } else {
                    Jid::from_phone_no(phone)
                };
                if is_self {
                    self.db.config.self_jid = Some(jid.clone());
                    self.db.config_autosave_free = true;
                }

                let should_add = self.db.contacts.get(&jid).is_none_or(|n| n.is_incomplete);
                if should_add {
                    return Ok(self.db.add_contact(Contact {
                        name,
                        jid: jid.to_id(),
                        muted: false,
                        is_group,
                        chatted: false,
                        last_message_time: Time(0),
                        last_read_message_time: Time(0),
                        is_incomplete: false,
                        last_msg_id: None,
                    })?);
                }
            }
            ChatEvent::NewChatsNotify {
                is_unread,
                is_muted,
                is_pinned,
                last_message_time,
            } => {
                println!("CHAT {id:?}: {last_message_time}");
                return Ok(self.e_new_chat_notify(
                    &id,
                    is_unread,
                    is_muted,
                    is_pinned,
                    last_message_time,
                )?);
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
            } => {
                let should_update_window =
                    if let (State::Chats(_, Some(ui)), Some(last_message_time)) = (
                        &self.state,
                        self.db.contacts.get(&id).map(|n| n.last_message_time),
                    ) {
                        if ui.selected == id && ui.chat_buffer.end_ts == last_message_time {
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                let t1 = self.db.add_message(MsgData {
                    content: text,
                    sender: sender_id.to_id(),
                    source: id.to_id(),
                    replying_to: quoted_id.map(|n| n.0),
                    timestamp: Time(time_sent as u64),
                    msg_id: msg_id.0,
                    is_edited,
                    is_read,
                    from_me,
                })?;

                if should_update_window {
                    if let State::Chats(_, Some(ui)) = &mut self.state {
                        let t2 = ui.chat_buffer.load_begin(&self.db, false)?;
                        return Ok(Task::batch([t1, t2]));
                    }
                }
                return Ok(t1);
            }
            ChatEvent::NewTypingNotify { user_id, is_typing } => {
                if is_typing {
                    self.typing.insert(id, user_id);
                } else {
                    self.typing.remove(&user_id);
                }
            }
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

    fn e_new_chat_notify(
        &mut self,
        id: &Jid,
        is_unread: bool,
        is_muted: bool,
        is_pinned: bool,
        last_message_time: isize,
    ) -> Result<Task<Message>, String> {
        let task = self.db.operate_on_contact(id, |n, db| {
            n.chatted = true;
            n.muted = is_muted;
            let time = Time(last_message_time as u64);
            if n.last_message_time < time {
                n.last_message_time = time;

                let db = db.clone();
                let jid_s = id.to_id();

                _ = tokio::spawn(async move {
                    let time = time.0 as i64;
                    let _: Result<_, _> = sqlx::query!(
                        "UPDATE contacts SET last_message_time = ? WHERE jid = ?",
                        time,
                        jid_s
                    )
                    .execute(&db)
                    .await;
                });
            }
            if n.last_read_message_time < time && !is_unread {
                n.last_read_message_time = time;

                let db = db.clone();
                let jid_s = id.to_id();

                _ = tokio::spawn(async move {
                    let time = time.0 as i64;
                    let _: Result<_, _> = sqlx::query!(
                        "UPDATE contacts SET last_read_message_time = ? WHERE jid = ?",
                        time,
                        jid_s
                    )
                    .execute(&db)
                    .await;
                });
            }
        })?;

        self.db.sort_contacts();
        self.db.add_pin(id.clone(), is_pinned);
        Ok(task)
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
            |n| Message::Done(n.strerr().flatten()),
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
