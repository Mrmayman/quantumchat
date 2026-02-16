use std::{collections::HashMap, path::PathBuf, sync::LazyLock};

use whatsmeow_nchat::Jid;

use crate::{
    core::IntoStringError,
    storage::{config::Config, contact::Contact},
};

pub mod config;
pub mod contact;
// pub mod message;

pub static DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    let p = dirs::data_dir().unwrap().join("QuantumChat");
    _ = std::fs::create_dir_all(&p);
    p
});

pub struct Data {
    db: sled::Db,

    pub contacts: HashMap<String, Contact>,
    pub contacts_tree: sled::Tree,
    pub messages_tiebreaker: u32,
    pub messages_tree: sled::Tree,
    pub config: Config,
    pub order: Vec<Jid>,
}

impl Data {
    pub fn new() -> Result<Self, String> {
        let db = sled::open(DIR.join("data")).strerr()?;
        let contacts_tree = db.open_tree("contacts").strerr()?;
        let messages_tree = db.open_tree("messages").strerr()?;

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
            .map(|n| Jid(n))
            .filter(|n| !config.pins.contains(n))
            .collect();

        Ok(Data {
            db,
            contacts,
            contacts_tree,
            messages_tree,
            messages_tiebreaker: 0,
            config,
            order,
        })
    }
}
