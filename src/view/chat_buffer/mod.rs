use std::collections::VecDeque;

use iced::{Task, widget::scrollable::Viewport};
use whatsmeow_nchat::{Jid, MsgId};

use crate::{
    Message, jid,
    storage::{Data, Time},
    view::{chat_buffer::db_load::load_chats_from_db, rich_text::rich_text},
};

mod db_load;
pub use db_load::DbLoadResult;

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
        let task = t.load_begin(db, false).chain(t.load_begin(db, true));
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

    pub fn load_begin(&mut self, db: &Data, reverse: bool) -> Task<Message> {
        let timestamp = if reverse { self.start_ts } else { self.end_ts };
        let viewing = self.viewing_id.clone();
        let db = db.db.clone();

        Task::perform(
            async move { load_chats_from_db(reverse, timestamp, viewing, db).await },
            Message::ChatBufferLoaded,
        )
    }

    pub fn loaded(&mut self, db: &Data, mut r: DbLoadResult) -> Result<(), String> {
        if r.is_reverse {
            self.debounce_up = false;
        } else {
            self.debounce_down = false;
        }

        let mut ts = Time(0);

        for message in r.messages {
            ts = message.timestamp;

            let rendered = RenderedMessage {
                message: RMessageCore {
                    text: rich_text(&message.content),
                    // TODO: collapse name for multiple messages in a row
                    sender_name: db.display_jid(&jid!(message.sender)).to_owned(),
                    sender: jid!(message.sender),
                    id: MsgId(message.msg_id.clone()),
                },
                replying_to: r
                    .replies
                    .remove(&message.msg_id)
                    .and_then(|reply| Jid::parse(&reply.sender).map(|jid| (reply, jid)))
                    .map(|(reply, sender)| RMessageCore {
                        text: rich_text(&reply.content),
                        sender_name: db.display_jid(&sender).to_owned(),
                        sender,
                        id: MsgId(reply.msg_id),
                    }),
                time_display: message.timestamp.to_string(),
                timestamp: message.timestamp,
                is_edited: message.is_edited,
                from_me: message.from_me,
                reactions: r
                    .reactions
                    .extract_if(.., |n| n.message_id == message.msg_id)
                    .filter_map(|reaction| {
                        Jid::parse(&reaction.sender_id).map(|jid| (reaction, jid))
                    })
                    .map(|(rn, sender)| RenderedReaction {
                        sender_name: db.display_jid(&sender).to_owned(),
                        sender,
                        emoji: rn.emoji,
                        from_me: rn.from_me,
                    })
                    .collect(),
                hide_sender: false,
            };
            if r.is_reverse {
                self.messages.push_front(rendered);
            } else {
                self.messages.push_back(rendered);
            }
        }

        for i in 0..self.messages.len() {
            if i == 0 {
                continue;
            }
            if self.messages[i].message.sender == self.messages[i - 1].message.sender {
                self.messages[i].hide_sender = true;
            }
        }

        if r.is_reverse {
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
            if let Some(last) = self.messages.back()
                && self.end_ts > last.timestamp
            {
                self.end_ts = last.timestamp;
            }
        } else if let Some(first) = self.messages.front()
            && self.start_ts < first.timestamp
        {
            self.start_ts = first.timestamp;
        }
    }

    pub fn add_reaction(
        &mut self,
        db: &Data,
        msg_id: &whatsmeow_nchat::MsgId,
        emoji: String,
        sender: Jid,
        from_me: bool,
    ) {
        let Some(msg) = self.messages.iter_mut().find(|m| &m.message.id == msg_id) else {
            return;
        };
        msg.reactions.push(RenderedReaction {
            sender_name: db.display_jid(&sender).to_owned(),
            sender,
            emoji,
            from_me,
        });
    }
}

pub struct RenderedMessage {
    pub message: RMessageCore,
    pub replying_to: Option<RMessageCore>,
    pub reactions: Vec<RenderedReaction>,

    pub time_display: String,
    pub timestamp: Time,

    pub is_edited: bool,
    pub hide_sender: bool,
    pub from_me: bool,
}

pub struct RenderedReaction {
    pub sender_name: String,
    pub sender: Jid,
    pub emoji: String,
    pub from_me: bool,
}

#[derive(Clone, Debug)]
pub struct RMessageCore {
    pub text: Vec<iced::widget::text::Span<'static>>,
    pub id: MsgId,
    pub sender: Jid,
    pub sender_name: String,
}
