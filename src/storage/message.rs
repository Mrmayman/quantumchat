use iced::Task;
use sqlx::prelude::FromRow;

use crate::{
    jid,
    storage::{Data, Time},
    IntoStringError, Message,
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
        self.contacts_sort_free = true;
        let time = msg.timestamp;
        let t_contact = self.update_last_message(&msg, time)?;

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

    fn update_last_message(&mut self, msg: &MsgData, time: Time) -> Result<Task<Message>, String> {
        let t_contact = self.operate_on_contact(&jid!(msg.source), |n, db| {
            if n.last_message_time < time {
                n.last_message_time = time;

                let db = db.clone();
                let jid_s = msg.source.clone();

                let message_contents = msg.content.clone();
                n.last_msg_contents = Some(msg.content.clone());
                let message_sender = msg.sender.clone();
                n.last_msg_sender = Some(msg.sender.clone());

                _ = tokio::spawn(async move {
                    let time = time.0 as i64;
                    let _: Result<_, _> = sqlx::query!(
                        "UPDATE contacts SET last_message_time = ? WHERE jid = ?",
                        time,
                        jid_s
                    )
                    .execute(&db)
                    .await;
                    let _: Result<_, _> = sqlx::query!(
                        "UPDATE contacts SET last_msg_contents = ? WHERE jid = ?",
                        message_contents,
                        jid_s
                    )
                    .execute(&db)
                    .await;
                    let _: Result<_, _> = sqlx::query!(
                        "UPDATE contacts SET last_msg_sender = ? WHERE jid = ?",
                        message_sender,
                        jid_s
                    )
                    .execute(&db)
                    .await;
                });
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
        Ok(t_contact)
    }
}
