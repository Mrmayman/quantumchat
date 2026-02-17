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

    #[serde(rename = "ism")]
    pub muted: bool,
    #[serde(rename = "isc")]
    pub chatted: bool,
    #[serde(rename = "isg")]
    pub is_group: bool,
    #[serde(rename = "isi")]
    pub is_incomplete: bool,

    #[serde(rename = "lrmt")]
    pub last_read_message_time: u64,
    #[serde(rename = "lmt")]
    pub last_message_time: u64,
    #[serde(skip)]
    pub last_msg: Option<(Jid, String, String)>,
}

impl Data {
    pub fn add_contact(&mut self, contact: Contact) -> Result<(), String> {
        if contact.jid.server().to_string() == "lid" {
            println!("lid");
            self.add_contact_lid(&contact)?;
            return Ok(());
        }

        if !self.config.pins.contains(&contact.jid) && !self.order.contains(&contact.jid) {
            self.order.push(contact.jid.clone());
        }
        let value = serde_json::to_vec(&contact).strerr()?;

        self.contacts_tree
            .insert(contact.jid.to_id(), value)
            .strerr()?;
        self.contacts.insert(contact.jid.clone(), contact);

        Ok(())
    }

    fn add_contact_lid(&mut self, contact: &Contact) -> Result<(), String> {
        let jid = Jid::from_phone_no(contact.name.clone());
        self.contacts_lid_tree
            .insert(contact.jid.to_id(), jid.to_id().as_bytes())
            .strerr()?;
        self.contacts_lid.insert(contact.jid.clone(), jid);
        Ok(())
    }

    pub fn operate_on_contact<F>(&mut self, jid: &Jid, operation: F) -> Result<(), String>
    where
        F: FnOnce(&mut Contact),
    {
        let jid_raw = jid.to_id();
        if let Some(contact) = self.contacts_tree.get(&jid_raw).strerr()? {
            let mut contact = serde_json::from_slice::<Contact>(&contact).strerr()?;
            operation(&mut contact);
            let value = serde_json::to_vec(&contact).strerr()?;
            self.contacts.insert(jid.clone(), contact);
            self.contacts_tree.insert(jid_raw, value).strerr()?;
        } else {
            if jid.server().to_string() == "lid" {
                return Ok(());
            }

            // Contact doesn't exist, likely a group
            let mut contact = Contact {
                name: jid.number().to_owned(),
                jid: jid.clone(),
                muted: false,
                is_group: false,
                chatted: true,
                last_message_time: 0,
                last_read_message_time: 0,
                last_msg: None,
                is_incomplete: true,
            };
            operation(&mut contact);
            let value = serde_json::to_vec(&contact).strerr()?;
            self.contacts_tree.insert(jid_raw.clone(), value).strerr()?;
            if !self.config.pins.contains(jid) && !self.order.contains(&contact.jid) {
                self.order.push(jid.clone());
            }
            self.contacts.insert(jid.clone(), contact);
        }

        Ok(())
    }
}
