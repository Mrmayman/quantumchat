use iced::Task;
use sqlx::FromRow;
use whatsmeow_nchat::Jid;

use crate::{
    jid,
    storage::{Data, Time},
    IntoStringError, Message,
};

#[derive(FromRow, Clone)]
pub struct Contact {
    /// Will try (in order):
    /// - Saved contact name
    /// - Display name (their profile)
    /// - Phone number
    pub jid: String, // PRIMARY KEY, TEXT
    pub name: String, // TEXT NOT NULL

    pub muted: bool,
    pub chatted: bool,
    pub is_group: bool,
    pub is_incomplete: bool,

    pub last_msg_id: Option<String>,

    // Timestamps: UNIX seconds
    #[sqlx(try_from = "i64")]
    pub last_read_message_time: Time, // INTEGER NOT NULL DEFAULT 0
    #[sqlx(try_from = "i64")]
    pub last_message_time: Time, // INTEGER NOT NULL DEFAULT 0
}

impl Data {
    pub fn add_contact(&mut self, contact: Contact) -> Result<Task<Message>, String> {
        if contact.jid.ends_with("@lid") {
            return self.add_contact_lid(&contact);
        }

        let jid = jid!(contact.jid);

        if !self.config.pins.contains(&jid) && !self.order.contains(&jid) {
            self.order.push(jid.clone());
        }

        let task = self.db_update_contact(contact.clone());
        self.contacts.insert(jid, contact);

        Ok(task)
    }

    fn add_contact_lid(&mut self, contact: &Contact) -> Result<Task<Message>, String> {
        if contact.jid.contains("∙") {
            return Ok(Task::none());
        }
        let jid = Jid::from_phone_no(contact.name.clone());

        let from_jid = contact.jid.clone();
        let to_jid = jid.to_id();
        let db = self.db.clone();
        let t = Task::perform(
            async move {
                sqlx::query!(
                    "INSERT OR REPLACE INTO contacts_lid (from_jid, to_jid) VALUES (?, ?)",
                    from_jid,
                    to_jid,
                )
                .execute(&db)
                .await
            },
            |r| Message::Done(r.strerr().map(|_| ())),
        );

        self.contacts_lid.insert(jid!(contact.jid), jid);

        Ok(t)
    }

    pub fn operate_on_contact<F>(
        &mut self,
        jid: &Jid,
        operation: F,
    ) -> Result<Task<Message>, String>
    where
        F: FnOnce(&mut Contact, &sqlx::Pool<sqlx::Sqlite>),
    {
        if let Some(contact) = self.contacts.get_mut(jid) {
            operation(contact, &self.db);
            return Ok(Task::none());
        } else if jid.server().to_string() == "lid" {
            return Ok(Task::none());
        }

        // Create new fallback contact if it doesn't exist
        let mut contact = Contact {
            name: jid.number().to_owned(),
            jid: jid.to_id(),
            muted: false,
            is_group: false,
            chatted: true,
            last_message_time: Time(0),
            last_read_message_time: Time(0),
            is_incomplete: true,
            last_msg_id: None,
        };
        operation(&mut contact, &self.db);

        let task = self.db_update_contact(contact.clone());

        self.contacts.insert(jid.clone(), contact);
        Ok(task)
    }

    fn db_update_contact(&mut self, contact: Contact) -> Task<Message> {
        let db = self.db.clone();

        let fut = async move {
            let lmt = contact.last_message_time.0 as i64;
            let lrmt = contact.last_read_message_time.0 as i64;
            sqlx::query!(
                r"INSERT OR REPLACE INTO contacts (
                    jid, name,
                    muted, chatted, is_group, is_incomplete,
                    last_message_time, last_read_message_time
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                contact.jid,
                contact.name,
                contact.muted,
                contact.chatted,
                contact.is_group,
                contact.is_incomplete,
                lmt,
                lrmt,
            )
            .execute(&db)
            .await
        };

        Task::perform(fut, |n| Message::Done(n.strerr().map(|_| ())))
    }
}
