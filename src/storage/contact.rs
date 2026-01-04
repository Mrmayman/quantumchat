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
    pub display_name: Option<String>,
    pub jid: Jid,
    pub lid_jid: Option<String>,
    pub muted: bool,
}

impl Contact {
    pub fn get_render_name(&self) -> String {
        self.full_name
            .clone()
            .or_else(|| self.first_name.clone())
            .or_else(|| self.display_name.clone())
            .unwrap_or(self.jid.user.clone())
    }
}

impl From<ContactUpdate> for Contact {
    fn from(event: ContactUpdate) -> Self {
        Contact {
            full_name: event.action.full_name,
            first_name: event.action.first_name,
            display_name: event.action.username,
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
        let contact = Contact::from(contact);
        let key = contact.jid.as_key_str();
        self.contacts.insert(key.clone(), contact.clone());
        if !self.config.pins.contains(&contact.jid) && !self.order.contains(&contact.jid) {
            self.order.push(contact.jid.clone());
        }
        let value = serde_json::to_vec(&contact).strerr()?;

        self.contacts_tree.insert(key, value).strerr()?;

        Ok(())
    }

    pub fn operate_on_contact<F>(&mut self, jid: Jid, operation: F) -> Result<(), String>
    where
        F: FnOnce(&mut Contact),
    {
        let key = jid.as_key_str();

        if let Some(contact) = self.contacts_tree.get(&key).strerr()? {
            let mut contact = serde_json::from_slice::<Contact>(&contact).strerr()?;
            operation(&mut contact);
            let value = serde_json::to_vec(&contact).strerr()?;
            self.contacts.insert(key.clone(), contact.clone());
            self.contacts_tree.insert(key, value).strerr()?;
        } else {
            // Contact doesn't exist, likely a group
            let mut contact = Contact {
                full_name: None,
                first_name: None,
                display_name: None,
                jid,
                lid_jid: None,
                muted: false,
            };
            operation(&mut contact);
            self.contacts.insert(key.clone(), contact.clone());
            let value = serde_json::to_vec(&contact).strerr()?;
            self.contacts_tree.insert(key, value).strerr()?;
        }

        Ok(())
    }
}
