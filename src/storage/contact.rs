use serde::{Deserialize, Serialize};
use whatsapp_rust::types::events::ContactUpdate;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Jid {
    pub user: String,
    /// `s.whatsapp.net` for DM, `g.us` for group
    pub server: String,
}

impl Jid {
    pub fn as_key_str(&self) -> String {
        format!("{}@{}", self.user, self.server)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Contact {
    pub full_name: Option<String>,
    pub first_name: Option<String>,
    pub jid: Jid,
    pub lid_jid: Option<String>,
    pub muted: bool,
}

impl From<ContactUpdate> for Contact {
    fn from(event: ContactUpdate) -> Self {
        Contact {
            full_name: event.action.full_name,
            first_name: event.action.first_name,
            jid: Jid {
                user: event.jid.user,
                server: event.jid.server,
            },
            lid_jid: event.action.lid_jid,
            muted: false,
        }
    }
}
