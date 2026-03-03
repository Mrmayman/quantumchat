use serde::{Deserialize, Serialize};
use whatsmeow_nchat::Jid;

use crate::{
    storage::{Data, DIR},
    IntoStringError,
};

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub pins: Vec<Jid>,
    pub self_jid: Option<Jid>,
}

impl Config {
    pub fn load() -> Result<Self, String> {
        let p = DIR.join("config.json");
        match std::fs::read_to_string(&p) {
            Ok(n) => Ok(serde_json::from_str(&n).strerr()?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let new_config = Self {
                    pins: Vec::new(),
                    self_jid: None,
                };
                let config_str = serde_json::to_string(&new_config).strerr()?;

                std::fs::write(&p, &config_str).strerr()?;
                Ok(new_config)
            }
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn is_self(&self, jid: &Jid) -> bool {
        self.self_jid.as_ref().is_some_and(|n| n == jid)
    }
}

impl Data {
    pub fn add_pin(&mut self, pin: Jid, pinned: bool) {
        if pinned {
            if !self.config.pins.contains(&pin) {
                self.order.retain(|jid| *jid != pin);
                self.config.pins.push(pin);
            }
        } else {
            self.config.pins.retain(|p| *p != pin);
            if !self.order.contains(&pin) {
                self.order.insert(0, pin);
            }
        }
        self.config_autosave_free = true;
    }
}
