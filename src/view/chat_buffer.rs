use std::collections::VecDeque;

use whatsmeow_nchat::Jid;

use crate::{
    core::IntoStringError,
    storage::{message::MsgData, Data},
};

const MSG_LOAD_LIMIT: usize = 200;
const MSG_LIMIT: usize = 600;

pub struct ChatBuffer {
    pub messages: VecDeque<RenderedMessage>,
    pub start_ts: u64,
    pub end_ts: u64,
    pub viewing: Jid,
    pub viewing_id: String,
}

impl ChatBuffer {
    pub fn new(db: &Data, chat: Jid) -> Result<Self, String> {
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
        };
        t.load(db, true)?;
        t.load(db, false)?;
        Ok(t)
    }

    pub fn load(&mut self, db: &Data, reverse: bool) -> Result<(), String> {
        const DAY_SECS: u64 = 24 * 60 * 60;
        let r1 = Data::message_schema(
            &self.viewing_id,
            if reverse {
                if self.start_ts > DAY_SECS {
                    self.start_ts - DAY_SECS
                } else {
                    self.start_ts
                }
            } else {
                self.end_ts
            },
        );
        let r2 = Data::message_schema(
            &self.viewing_id,
            if reverse {
                self.start_ts
            } else {
                self.end_ts + DAY_SECS
            },
        );
        let loaded = || db.messages_list_tree.range(r1.as_slice()..r2.as_slice());
        println!("loaded {} {reverse}", loaded().count());
        let loaded = loaded();

        if reverse {
            let mut start_ts = 0;
            for m in loaded.rev().take(MSG_LOAD_LIMIT) {
                let Some(msg_data) = db.messages_tree.get(&m.strerr()?.1).strerr()? else {
                    continue;
                };
                let message: MsgData = serde_json::from_slice(&msg_data).strerr()?;
                start_ts = message.timestamp;
                let rendered = msgdata_to_rendered(db, message)?;
                self.messages.push_front(rendered);
            }
            self.messages.truncate(MSG_LIMIT);
            self.start_ts = start_ts;
        } else {
            let mut end_ts = 0;
            for m in loaded.take(MSG_LOAD_LIMIT) {
                let Some(msg_data) = db.messages_tree.get(&m.strerr()?.1).strerr()? else {
                    continue;
                };
                let message: MsgData = serde_json::from_slice(&msg_data).strerr()?;
                end_ts = message.timestamp;
                let rendered = msgdata_to_rendered(db, message)?;
                if self
                    .messages
                    .iter()
                    .next_back()
                    .is_some_and(|n| n.message.text == rendered.message.text)
                {
                    continue;
                }
                self.messages.push_back(rendered);
            }
            let len = self.messages.len();
            if len > MSG_LIMIT {
                self.messages.drain(0..(len - MSG_LIMIT));
            }
            self.end_ts = end_ts;
        }

        Ok(())
    }
}

fn msgdata_to_rendered(db: &Data, message: MsgData) -> Result<RenderedMessage, String> {
    let sender = message.get_sender();
    let replying_to = if let Some(replying_id) = &message.replying_to {
        if let Some(msg_data) = db.messages_tree.get(&replying_id.0).strerr()? {
            let message: MsgData = serde_json::from_slice(&msg_data).strerr()?;
            let sender = message.get_sender();
            Some(RMessageCore {
                sender_name: db.display_jid(sender).to_owned(),
                sender: sender.clone(),
                text: message.c,
            })
        } else {
            None
        }
    } else {
        None
    };
    let message = RenderedMessage {
        message: RMessageCore {
            text: message.c.clone(),
            sender_name: db.display_jid(sender).to_owned(),
            sender: sender.clone(),
        },
        replying_to,
        time_display: "".to_owned(), // TODO
        is_edited: message.is_edited,
        from_me: message.from_me,
    };
    Ok(message)
}

pub struct RenderedMessage {
    pub message: RMessageCore,
    pub replying_to: Option<RMessageCore>,
    pub time_display: String,
    pub is_edited: bool,
    pub from_me: bool,
}

pub struct RMessageCore {
    pub text: String,
    pub sender: Jid,
    pub sender_name: String,
}
