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
    pub timestamp: i64,
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

impl Data {
    pub fn add_message(&mut self, msg: MsgData) -> Result<(), String> {
        self.messages_tree
            .insert(&msg.msg_id.0, serde_json::to_vec(&msg).strerr()?)
            .strerr()?;

        let id = msg.src.to_id();
        let mut key = id.len().to_be_bytes().to_vec();
        key.extend(id.as_bytes());
        key.extend(msg.timestamp.to_be_bytes());
        key.extend(msg.msg_id.0.as_bytes());

        self.messages_list_tree
            .insert(key, msg.msg_id.0.as_bytes())
            .strerr()?;

        Ok(())
    }
}
