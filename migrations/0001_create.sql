-- All timestamps are unix time (in milliseconds? idk)

create table if not exists contacts
(
    jid TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,

    -- Booleans (stored as INTEGER 0/1 in SQLite)
    muted BOOLEAN NOT NULL DEFAULT 0,
    chatted BOOLEAN NOT NULL DEFAULT 0,
    is_group BOOLEAN NOT NULL DEFAULT 0,
    is_incomplete BOOLEAN NOT NULL DEFAULT 1,

    -- Timestamps (UNIX seconds)
    last_read_message_time INTEGER NOT NULL DEFAULT (strftime('%s','now')),
    last_message_time      INTEGER NOT NULL DEFAULT (strftime('%s','now')),

    last_msg_contents TEXT,
    last_msg_sender TEXT
    -- FOREIGN KEY (last_msg_id) REFERENCES messages(msg_id)
);

create table if not exists contacts_lid
(
    from_jid TEXT PRIMARY KEY NOT NULL,
    to_jid TEXT NOT NULL
    -- FOREIGN KEY (to_jid) REFERENCES contacts(jid)
);

create table if not exists messages
(
    msg_id TEXT PRIMARY KEY NOT NULL,
    content TEXT NOT NULL,

    source TEXT NOT NULL,            -- source Jid (group ID or sender)
    sender TEXT NOT NULL,            -- sender Jid if group, else same as source

    timestamp INTEGER NOT NULL,

    is_edited BOOLEAN NOT NULL DEFAULT 0,
    is_read BOOLEAN NOT NULL DEFAULT 0,
    from_me BOOLEAN NOT NULL DEFAULT 0,

    replying_to TEXT                -- references msg_id if reply
    -- FOREIGN KEY (replying_to) REFERENCES messages(msg_id)
);
