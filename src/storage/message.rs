use iced::Task;
use sqlx::prelude::FromRow;
use whatsmeow_nchat::{Jid, MsgId};

use crate::{
    core::IntoStringError,
    jid,
    storage::{Data, Time},
    Message,
};

#[derive(Debug, Clone, FromRow)]
pub struct MsgData {
    pub msg_id: String,  // PRIMARY KEY, TEXT
    pub content: String, // TEXT NOT NULL

    pub source: String, // source Jid (group ID or sender)
    pub sender: String, // sender Jid if group, else same as source

    #[sqlx(try_from = "i64")]
    pub timestamp: Time, // INTEGER NOT NULL, Unix time in milliseconds

    pub is_edited: bool,
    pub is_read: bool,
    pub from_me: bool,

    pub replying_to: Option<String>, // TEXT, nullable, references msg_id
}

impl Data {
    pub fn add_message(&mut self, msg: MsgData) -> Result<Task<Message>, String> {
        let time = msg.timestamp;
        let t_contact = self.operate_on_contact(&jid!(msg.source), |n, db| {
            if n.last_message_time < time {
                n.last_message_time = time;

                let db = db.clone();
                let jid_s = msg.source.clone();

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

                // n.last_msg = Some((msg.sender.clone(), msg.c.clone(), "4:20".to_owned()));
                // TODO time display
            }
            if msg.is_read && n.last_read_message_time < time {
                n.last_read_message_time = time;

                let db = db.clone();
                let jid_s = msg.source.clone();

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

        let db = self.db.clone();
        let t_msg = Task::perform(
            async move {
                let timestamp = msg.timestamp.0 as i64;
                sqlx::query!(
                    r"INSERT OR REPLACE INTO messages (
                msg_id, content,
                source, sender,
                timestamp, replying_to,
                is_edited, is_read, from_me
            ) values (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    msg.msg_id,
                    msg.content,
                    msg.source,
                    msg.sender,
                    timestamp,
                    msg.replying_to,
                    msg.is_edited,
                    msg.is_read,
                    msg.from_me,
                )
                .execute(&db)
                .await
            },
            |r| Message::Done(r.strerr().map(|_| ())),
        );
        Ok(Task::batch([t_contact, t_msg]))
    }

    pub fn get_last_message(&self, id: &Jid) -> Option<MsgId> {
        // let chat_id = id.to_id();
        // let mut key = chat_id.len().to_be_bytes().to_vec();
        // key.extend(chat_id.as_bytes());

        // let scan = || self.messages_list_tree.scan_prefix(&key);

        // scan()
        //     .values()
        //     .next_back()
        //     .map(|n| n.ok())
        //     .flatten()
        //     .map(|n| MsgId(String::from_utf8_lossy(&n).to_string()))
        None
    }

    pub fn update_last_messages(&mut self) {
        // let mut to_update = HashMap::new();
        for jid in self.contacts.keys() {
            let Some(last_msg) = self.get_last_message(jid) else {
                continue;
            };
            // let Some(msg) = self.get_message(&last_msg) else {
            //     continue;
            // };
            // to_update.insert(jid.clone(), msg);
        }
        // for (id, msg) in to_update {
        //     let Some(contact) = self.contacts.get_mut(&id) else {
        //         continue;
        //     };
        // contact.last_msg = Some((msg.get_sender().clone(), msg.c, "4:20".to_owned()));
        // }
    }
}
