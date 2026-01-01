use serde::{Deserialize, Serialize};
use whatsapp_rust::types::events::ContactUpdate;

use crate::{core::IntoStringError, storage::Data};

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

    pub fn from_key(key: &str) -> Self {
        let mut split = key.split('@');
        Jid {
            user: split.next().unwrap().to_owned(),
            server: split.next().unwrap().to_owned(),
        }
    }
}

impl From<wacore_binary::jid::Jid> for Jid {
    fn from(jid: wacore_binary::jid::Jid) -> Self {
        Jid {
            user: jid.user,
            server: jid.server,
        }
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

impl Contact {
    pub fn get_render_name(&self) -> String {
        self.full_name
            .clone()
            .unwrap_or(self.first_name.clone().unwrap_or(self.jid.user.clone()))
    }
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

impl Data {
    pub fn add_contact(&mut self, contact: ContactUpdate) -> Result<(), String> {
        let tree = self.db.open_tree("contacts").strerr()?;

        let contact = Contact::from(contact);
        let key = contact.jid.as_key_str();
        self.contacts.insert(key.clone(), contact.clone());
        let value = serde_json::to_vec(&contact).strerr()?;

        tree.insert(key, value).strerr()?;

        Ok(())
    }

    pub fn add_mute(&mut self, jid: Jid, muted: bool) -> Result<(), String> {
        let tree = self.db.open_tree("contacts").strerr()?;
        let key = jid.as_key_str();

        if let Some(contact) = tree.get(&key).strerr()? {
            let mut contact = serde_json::from_slice::<Contact>(&contact).strerr()?;
            contact.muted = muted;
            let value = serde_json::to_vec(&contact).strerr()?;
            self.contacts.insert(key.clone(), contact.clone());
            tree.insert(key, value).strerr()?;
        } else {
            // Contact doesn't exist, likely a group
            let contact = Contact {
                full_name: None,
                first_name: None,
                jid,
                lid_jid: None,
                muted,
            };
            self.contacts.insert(key.clone(), contact.clone());
            let value = serde_json::to_vec(&contact).strerr()?;
            tree.insert(key, value).strerr()?;
        }

        Ok(())
    }
}
