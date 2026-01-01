use std::{collections::HashMap, path::PathBuf, sync::LazyLock};

use crate::{
    core::IntoStringError,
    storage::{
        config::Config,
        contact::{Contact, Jid},
    },
};

pub mod config;
pub mod contact;

pub static DIR: LazyLock<PathBuf> = LazyLock::new(|| dirs::data_dir().unwrap().join("QuantumChat"));

pub struct Data {
    db: sled::Db,
    pub contacts: HashMap<String, Contact>,
    pub config: Config,
    pub order: Vec<Jid>,
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

        let config = if let Ok(Some(config)) = db.get("config") {
            serde_json::from_slice::<Config>(&config).strerr()?
        } else {
            let config = Config { pins: Vec::new() };
            db.insert("config", serde_json::to_vec(&config).strerr()?)
                .strerr()?;
            config
        };

        let order = contacts
            .keys()
            .cloned()
            .map(|n| Jid::from_key(&n))
            .filter(|n| !config.pins.contains(n))
            .collect();

        Ok(Data {
            db,
            contacts,
            config,
            order,
        })
    }
}
