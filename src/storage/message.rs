use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use whatsmeow_nchat::{Jid, MsgId};

use crate::{core::IntoStringError, storage::Data};

#[derive(Serialize, Deserialize, Clone)]
pub struct MsgData {
    pub c: String,
    /// If group then group ID, else same as `sender`
    pub src: Jid,
    /// If group then sender's ID, else same as `src`
    #[serde(rename = "snd")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender: Option<Jid>,

    #[serde(rename = "reply")]
    pub replying_to: Option<MsgId>,
    #[serde(rename = "time")]
    pub timestamp: u64,
    pub msg_id: MsgId,
    #[serde(rename = "ise")]
    pub is_edited: bool,
    #[serde(rename = "isr")]
    pub is_read: bool,
    #[serde(rename = "ism")]
    pub from_me: bool,
    // file_id,
    // file_path,
    // file_status,
}

impl MsgData {
    pub fn get_sender(&self) -> &Jid {
        self.sender.as_ref().unwrap_or(&self.src)
    }
}

impl Data {
    pub fn add_message(&mut self, msg: MsgData) -> Result<(), String> {
        let time = msg.timestamp;
        self.operate_on_contact(&msg.src, |n| {
            if n.last_message_time < time {
                n.last_message_time = time;
                n.last_msg = Some((msg.get_sender().clone(), msg.c.clone(), "4:20".to_owned()));
                // TODO time display
            }
            if msg.is_read && n.last_read_message_time < time {
                n.last_read_message_time = time;
            }
        })?;
        self.messages_tree
            .insert(&msg.msg_id.0, serde_json::to_vec(&msg).strerr()?)
            .strerr()?;

        let mut key = Self::message_schema(&msg.src.to_id(), msg.timestamp);
        key.extend(msg.msg_id.0.as_bytes());

        self.messages_list_tree
            .insert(key, msg.msg_id.0.as_bytes())
            .strerr()?;

        Ok(())
    }

    pub fn message_schema(chat_id: &str, timestamp: u64) -> Vec<u8> {
        let mut key = chat_id.len().to_be_bytes().to_vec();
        key.extend(chat_id.as_bytes());
        key.extend(timestamp.to_be_bytes());
        key
    }

    pub fn get_last_message(&self, id: &Jid) -> Option<MsgId> {
        let chat_id = id.to_id();
        let mut key = chat_id.len().to_be_bytes().to_vec();
        key.extend(chat_id.as_bytes());

        let scan = || self.messages_list_tree.scan_prefix(&key);

        scan()
            .values()
            .next_back()
            .map(|n| n.ok())
            .flatten()
            .map(|n| MsgId(String::from_utf8_lossy(&n).to_string()))
    }

    pub fn get_message(&self, msg_id: &MsgId) -> Option<MsgData> {
        let msg = self.messages_tree.get(&msg_id.0).ok()??;
        let msg: MsgData = serde_json::from_slice(&msg).ok()?;
        Some(msg)
    }

    pub fn update_last_messages(&mut self) {
        let mut to_update = HashMap::new();
        for jid in self.contacts.keys() {
            let Some(last_msg) = self.get_last_message(jid) else {
                continue;
            };
            let Some(msg) = self.get_message(&last_msg) else {
                continue;
            };
            to_update.insert(jid.clone(), msg);
        }
        for (id, msg) in to_update {
            let Some(contact) = self.contacts.get_mut(&id) else {
                continue;
            };
            contact.last_msg = Some((msg.get_sender().clone(), msg.c, "4:20".to_owned()));
        }
    }
}
