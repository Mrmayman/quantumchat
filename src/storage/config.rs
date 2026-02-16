use serde::{Deserialize, Serialize};
use whatsmeow_nchat::Jid;

use crate::{core::IntoStringError, storage::Data};

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub pins: Vec<Jid>,
}

impl Data {
    pub fn add_pin(&mut self, pin: Jid, pinned: bool) -> Result<(), String> {
        if pinned {
            if !self.config.pins.contains(&pin) {
                self.order.retain(|jid| *jid != pin);
                self.config.pins.push(pin);
            }
        } else {
            self.config.pins.retain(|p| *p != pin);
            self.order.insert(0, pin);
        }
        self.save_config().strerr()
    }

    pub fn save_config(&mut self) -> Result<(), String> {
        self.db
            .insert("config", serde_json::to_vec(&self.config).strerr()?)
            .strerr()?;
        Ok(())
    }
}
