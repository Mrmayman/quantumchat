use std::{collections::HashMap, path::PathBuf, sync::LazyLock};

use whatsapp_rust::types::events::ContactUpdate;

use crate::{
    core::IntoStringError,
    storage::contact::{Contact, Jid},
};

pub mod contact;

pub static DIR: LazyLock<PathBuf> = LazyLock::new(|| dirs::data_dir().unwrap().join("QuantumChat"));

pub struct Data {
    db: sled::Db,
    contacts: HashMap<String, Contact>,
}

impl Data {
    pub fn new() -> Result<Self, String> {
        let db = sled::open(DIR.join("data")).strerr()?;
        let contacts_tree = db.open_tree("contacts").strerr()?;

        let contacts = contacts_tree
            .iter()
            .map(|r| {
                let (k, v) = r.strerr()?;
                Ok::<_, String>((
                    String::from_utf8_lossy(&k).to_string(),
                    serde_json::from_slice::<Contact>(&v).strerr()?,
                ))
            })
            .collect::<Result<HashMap<String, Contact>, String>>()?;

        Ok(Data { db, contacts })
    }

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
