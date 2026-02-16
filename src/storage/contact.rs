use serde::{Deserialize, Serialize};
use whatsmeow_nchat::Jid;

use crate::{core::IntoStringError, storage::Data};

#[derive(Serialize, Deserialize, Clone)]
pub struct Contact {
    /// Will try (in order):
    /// - Saved contact name
    /// - Display name (their profile)
    /// - Phone number
    pub name: String,
    pub jid: Jid,
    pub muted: bool,
    pub is_group: bool,
}

impl Data {
    pub fn add_contact(&mut self, contact: Contact) -> Result<(), String> {
        if !self.config.pins.contains(&contact.jid) && !self.order.contains(&contact.jid) {
            self.order.push(contact.jid.clone());
        }
        let value = serde_json::to_vec(&contact).strerr()?;

        self.contacts_tree
            .insert(contact.jid.0.clone(), value)
            .strerr()?;
        self.contacts.insert(contact.jid.0.clone(), contact);

        Ok(())
    }

    pub fn operate_on_contact<F>(&mut self, jid: Jid, operation: F) -> Result<(), String>
    where
        F: FnOnce(&mut Contact),
    {
        if let Some(contact) = self.contacts_tree.get(&jid.0).strerr()? {
            let mut contact = serde_json::from_slice::<Contact>(&contact).strerr()?;
            operation(&mut contact);
            let value = serde_json::to_vec(&contact).strerr()?;
            self.contacts.insert(jid.0.clone(), contact);
            self.contacts_tree.insert(jid.0, value).strerr()?;
        } else {
            // Contact doesn't exist, likely a group
            let mut contact = Contact {
                name: jid.0.split('@').next().unwrap_or(jid.0.as_str()).to_owned(),
                jid: jid.clone(),
                muted: false,
                is_group: false,
            };
            operation(&mut contact);
            let value = serde_json::to_vec(&contact).strerr()?;
            self.contacts_tree.insert(jid.0.clone(), value).strerr()?;
            self.contacts.insert(jid.0, contact);
        }

        Ok(())
    }
}
