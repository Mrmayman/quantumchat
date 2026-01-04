use serde::{Deserialize, Serialize};
use whatsapp_rust::types::message::MessageInfo;

use crate::{
    core::IntoStringError,
    storage::{Data, contact::Jid},
};

#[derive(Serialize, Deserialize, Clone)]
pub struct Msg {
    pub content: String,
    /// - `Some(_)`: Person who sent the message
    /// - `None`: Sent by me
    pub sender: Option<Jid>,
    /// - `Some(_)`: Group that message was sent in
    /// - `None`: Direct message
    pub chat: Option<Jid>,
    pub replying_to: Option<(Jid, String)>,
    pub timestamp: i64,
}

impl Data {
    pub fn add_message(
        &mut self,
        msg: &waproto::whatsapp::Message,
        msg_info: MessageInfo,
    ) -> Result<(), String> {
        let chat: Jid = msg_info.source.chat.into();
        let sender: Jid = msg_info.source.sender.into();
        let Some(content) = msg.conversation.clone().or_else(|| {
            msg.extended_text_message
                .as_deref()
                .and_then(|n| n.text.clone())
        }) else {
            return Ok(());
        };
        let is_group = chat != sender;
        let timestamp = msg_info.timestamp.timestamp();
        let msg = Msg {
            content,
            chat: is_group.then_some(chat.clone()),
            sender: (!msg_info.source.is_from_me).then_some(sender.clone()),
            replying_to: None, // TODO
            timestamp,
        };
        self.operate_on_contact(sender, |contact| {
            contact.display_name = Some(msg_info.push_name)
        })?;

        let mut key = chat.as_key_str().as_bytes().to_owned();
        key.extend((timestamp as u64).to_be_bytes());
        key.extend(self.messages_tiebreaker.to_be_bytes());

        let msg_json = serde_json::to_string(&msg).strerr()?;
        self.messages_tree
            .insert(key, msg_json.as_bytes())
            .strerr()?;
        self.messages_tiebreaker = self.messages_tiebreaker.wrapping_add_signed(1);

        Ok(())
    }
}
