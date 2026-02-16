use std::{collections::HashMap, path::PathBuf, sync::LazyLock};

use whatsmeow_nchat::Jid;

use crate::{
    core::IntoStringError,
    storage::{config::Config, contact::Contact},
};

pub mod config;
pub mod contact;
pub mod message;

pub static DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    let p = dirs::data_dir().unwrap().join("QuantumChat");
    _ = std::fs::create_dir_all(&p);
    p
});

pub struct Data {
    db: sled::Db,

    pub contacts: HashMap<String, Contact>,
    pub contacts_tree: sled::Tree,
    messages_tree: sled::Tree,
    messages_list_tree: sled::Tree,
    pub config: Config,
    pub order: Vec<Jid>,
}

impl Data {
    pub fn new() -> Result<Self, String> {
        const CACHE: u64 = 32 * 1024 * 1024;
        let db = sled::Config::new()
            .path(DIR.join("data"))
            .use_compression(true)
            .cache_capacity(CACHE)
            .compression_factor(2)
            .mode(sled::Mode::HighThroughput)
            .open()
            .strerr()?;
        let contacts_tree = db.open_tree("contacts").strerr()?;
        let messages_tree = db.open_tree("messages").strerr()?;
        let messages_list_tree = db.open_tree("messages_list").strerr()?;

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
            let config = Config {
                pins: Vec::new(),
                self_jid: None,
            };
            db.insert("config", serde_json::to_vec(&config).strerr()?)
                .strerr()?;
            config
        };

        let order: Vec<Jid> = contacts
            .keys()
            .cloned()
            .filter_map(|n| Jid::parse(&n))
            .filter(|n| !config.pins.contains(n))
            .collect();

        let mut data = Data {
            db,
            contacts,
            contacts_tree,
            messages_tree,
            messages_list_tree,
            config,
            order,
        };
        data.sort_contacts();
        Ok(data)
    }

    pub fn sort_contacts(&mut self) {
        self.order.sort_unstable_by(|a, b| {
            let (Some(ca), Some(cb)) =
                (self.contacts.get(&a.to_id()), self.contacts.get(&b.to_id()))
            else {
                return std::cmp::Ordering::Equal;
            };
            cb.last_message_time.cmp(&ca.last_message_time)
        });
    }
}
