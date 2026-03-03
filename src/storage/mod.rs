use std::{collections::HashMap, path::PathBuf, sync::LazyLock};

use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
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
    pub db: sqlx::SqlitePool,
    pub runtime: tokio::runtime::Runtime,

    pub contacts: HashMap<Jid, Contact>,
    pub contacts_lid: HashMap<Jid, Jid>,

    pub config: Config,
    pub config_autosave_free: bool,
    pub order: Vec<Jid>,
    pub latest_timestamp: Time,
}

impl Data {
    pub fn new() -> Result<Self, String> {
        let opts = SqliteConnectOptions::new()
            .filename(DIR.join("main.db"))
            .create_if_missing(true)
            .optimize_on_close(true, None)
            // .pragma("cache_size", "-16384")
            .statement_cache_capacity(4)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal);

        let runtime = tokio::runtime::Runtime::new().strerr()?;

        let db = runtime.block_on(SqlitePool::connect_with(opts)).strerr()?;
        // .use_compression(true)
        // .compression_factor(2)
        // .mode(sled::Mode::HighThroughput)

        runtime
            .block_on(sqlx::migrate!("./migrations").run(&db))
            .strerr()?;

        let contacts = runtime
            .block_on(sqlx::query_as!(Contact, "select * from contacts").fetch_all(&db))
            .strerr()?
            .into_iter()
            .map(|r| {
                Ok::<_, String>((Jid::parse(&r.jid).ok_or_else(|| "JID error".to_owned())?, r))
            })
            .collect::<Result<HashMap<Jid, Contact>, String>>()?;

        let contacts_lid = runtime
            .block_on(sqlx::query!("select * from contacts_lid").fetch_all(&db))
            .strerr()?
            .into_iter()
            .filter_map(|r| Some((Jid::parse(&r.from_jid)?, Jid::parse(&r.to_jid)?)))
            // Some people hide their numbers for privacy, those fail to parse
            // For example, +44∙∙∙∙∙∙∙∙85@s.whatsapp.net (with those dot characters)
            // So we do filter_map
            .collect::<HashMap<Jid, Jid>>();

        let config = Config::load()?;

        let order: Vec<Jid> = contacts
            .keys()
            .cloned()
            .filter(|n| !config.pins.contains(n))
            .collect();

        let mut data = Data {
            db,
            contacts,
            contacts_lid,
            config,
            order,
            runtime,
            latest_timestamp: Time(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|n| n.as_secs())
                    .unwrap_or_default(),
            ),
            config_autosave_free: false,
        };
        data.sort_contacts();
        Ok(data)
    }

    pub fn sort_contacts(&mut self) {
        self.order.sort_unstable_by(|a, b| {
            let (Some(ca), Some(cb)) = (self.contacts.get(&a), self.contacts.get(&b)) else {
                return std::cmp::Ordering::Equal;
            };
            cb.last_message_time.cmp(&ca.last_message_time)
        });
    }

    pub fn display_jid<'a>(&'a self, jid: &'a Jid) -> &'a str {
        self.contacts_lid
            .get(&jid)
            .and_then(|n| self.contacts.get(n))
            .or_else(|| self.contacts.get(&jid))
            .map_or(jid.number(), |n| &n.name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Time(pub u64);

impl From<i64> for Time {
    fn from(value: i64) -> Self {
        Self(value as u64)
    }
}
