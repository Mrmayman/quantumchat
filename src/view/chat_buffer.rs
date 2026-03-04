use std::collections::VecDeque;

use iced::{widget::scrollable::Viewport, Task};
use whatsmeow_nchat::Jid;

use crate::{
    jid,
    storage::{message::MsgData, Data, Time},
    IntoStringError, Message,
};

const MSG_LOAD_LIMIT: usize = 200;
const MSG_LIMIT: usize = 600;

pub struct ChatBuffer {
    pub messages: VecDeque<RenderedMessage>,
    pub start_ts: Time,
    pub end_ts: Time,
    pub viewing: Jid,
    pub viewing_id: String,

    pub scroll: Viewport,

    debounce_up: bool,
    debounce_down: bool,
}

impl ChatBuffer {
    pub fn new(db: &Data, chat: Jid) -> Result<(Self, Task<Message>), String> {
        let chat_id = chat.to_id();

        let timestamp = db
            .contacts
            .get(&chat)
            // .map(|n| n.last_read_message_time) // too buggy
            .map(|n| n.last_message_time)
            .unwrap_or(db.latest_timestamp);

        let mut t = Self {
            messages: VecDeque::new(),
            start_ts: timestamp,
            end_ts: timestamp,
            viewing: chat,
            viewing_id: chat_id,
            debounce_up: true,
            debounce_down: true,
            scroll: unsafe { std::mem::zeroed() }, // hear me out, I had no choice
        };
        let task = t.load_begin(db, false)?.chain(t.load_begin(db, true)?);
        Ok((t, task))
    }

    pub fn debounce(&mut self, reverse: bool) -> bool {
        if reverse {
            if self.debounce_up {
                return true;
            }
            self.debounce_up = true;
        } else {
            if self.debounce_down {
                return true;
            }
            self.debounce_down = true;
        }
        false
    }

    pub fn load_begin(&mut self, db: &Data, reverse: bool) -> Result<Task<Message>, String> {
        let timestamp = if reverse { self.start_ts } else { self.end_ts };
        let viewing = self.viewing_id.clone();
        let db = db.db.clone();

        Ok(Task::perform(
            async move {
                let time = timestamp.0 as i64;
                let q = if reverse {
                    sqlx::query_as!(
                    MsgData,
                    "SELECT * FROM messages WHERE source = ? AND timestamp < ? ORDER BY timestamp DESC LIMIT ?",
                    viewing,
                    time,
                    MSG_LOAD_LIMIT as i64
                ).fetch_all(&db).await
                } else {
                    sqlx::query_as!(
                    MsgData,
                    "SELECT * FROM messages WHERE source = ? AND timestamp > ? ORDER BY timestamp ASC LIMIT ?",
                    viewing,
                    time,
                    MSG_LOAD_LIMIT as i64
                ).fetch_all(&db).await
                };
                q.strerr()
            },
            move |r| Message::ChatBufferLoaded(r, reverse),
        ))

        // let mut start_ts = 0;
        // for m in loaded.rev().take(MSG_LOAD_LIMIT) {
        //     let Some(msg_data) = db.messages_tree.get(&m.strerr()?.1).strerr()? else {
        //         continue;
        //     };
        //     let message: MsgData = serde_json::from_slice(&msg_data).strerr()?;
        //     start_ts = message.timestamp;
        //     let rendered = msgdata_to_rendered(db, message)?;
        //     self.messages.push_front(rendered);
        // }
        // self.messages.truncate(MSG_LIMIT);
        // self.start_ts = start_ts;
    }

    pub fn loaded(
        &mut self,
        db: &Data,
        messages: Vec<MsgData>,
        reverse: bool,
    ) -> Result<(), String> {
        if reverse {
            self.debounce_up = false;
        } else {
            self.debounce_down = false;
        }

        let mut ts = Time(0);

        for message in messages {
            ts = message.timestamp;

            let rendered = RenderedMessage {
                message: RMessageCore {
                    text: message.content,
                    // TODO: collapse name for multiple messages in a row
                    sender_name: db.display_jid(&jid!(message.sender)).to_owned(),
                    sender: jid!(message.sender),
                },
                replying_to: None,           // TODO
                time_display: "".to_owned(), // TODO
                timestamp: message.timestamp,
                is_edited: message.is_edited,
                from_me: message.from_me,
            };
            if reverse {
                self.messages.push_front(rendered);
            } else {
                self.messages.push_back(rendered);
            }
        }

        if reverse {
            if self.start_ts.0 > ts.0 {
                self.start_ts = ts;
            }
        } else if self.end_ts.0 < ts.0 {
            self.end_ts = ts;
        }
        Ok(())
    }

    pub fn shrink(&mut self, messages: usize, reverse: bool) {
        while self.messages.len() + messages > MSG_LIMIT {
            if reverse {
                self.messages.pop_back();
            } else {
                self.messages.pop_front();
            }
        }
        if reverse {
            // We loaded from front (up), so removing from end
            if let Some(last) = self.messages.back() {
                if self.end_ts > last.timestamp {
                    self.end_ts = last.timestamp;
                }
            }
        } else if let Some(first) = self.messages.front() {
            if self.start_ts < first.timestamp {
                self.start_ts = first.timestamp;
            }
        }
    }
}

pub struct RenderedMessage {
    pub message: RMessageCore,
    pub replying_to: Option<RMessageCore>,
    pub time_display: String,
    pub timestamp: Time,

    pub is_edited: bool,
    pub from_me: bool,
}

pub struct RMessageCore {
    pub text: String,
    pub sender: Jid,
    pub sender_name: String,
}
