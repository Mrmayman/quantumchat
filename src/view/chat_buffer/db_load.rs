use std::collections::HashMap;

use crate::{
    core::IntoStringError,
    storage::{
        Time,
        message::{MsgData, ReactionData},
    },
};

const MSG_LOAD_LIMIT: usize = 200;

#[derive(Debug, Clone)]
pub struct DbLoadResult {
    pub messages: Vec<MsgData>,
    pub reactions: Vec<ReactionData>,
    /// Map from original message id to data of referenced message
    pub replies: HashMap<String, MsgData>,
    /// `true` -> Loading older messages (scrolling up), `false` -> Loading newer messages
    pub is_reverse: bool,
}

pub async fn load_chats_from_db(
    is_reverse: bool,
    timestamp: Time,
    viewing: String,
    db: sqlx::Pool<sqlx::Sqlite>,
) -> Result<DbLoadResult, String> {
    let time = timestamp.0 as i64;

    let messages = load_messages(&db, is_reverse, viewing, time).await?;
    let (reactions, replies) =
        tokio::try_join!(load_reactions(&db, &messages), load_replies(&db, &messages))?;

    Ok::<_, String>(DbLoadResult {
        messages,
        reactions,
        replies,
        is_reverse,
    })
}

async fn load_replies(
    db: &sqlx::Pool<sqlx::Sqlite>,
    messages: &Vec<MsgData>,
) -> Result<HashMap<String, MsgData>, String> {
    let mut replies = HashMap::new();
    let mut replies_to_load = HashMap::new();
    for msg in messages {
        if let Some(reply_msg_id) = &msg.replying_to {
            if let Some(reply_msg) = messages.iter().find(|m| m.msg_id == *reply_msg_id) {
                replies.insert(msg.msg_id.clone(), reply_msg.clone());
            } else {
                replies_to_load
                    .entry(reply_msg_id)
                    .or_insert(Vec::new())
                    .push(&msg.msg_id);
            }
        }
    }
    let mut query = "SELECT * FROM messages WHERE msg_id IN ".to_owned();
    query_append_list(&mut query, replies_to_load.len());
    let mut q = sqlx::query_as::<_, MsgData>(&query);
    for id in replies_to_load.keys() {
        q = q.bind(id);
    }
    let loaded_replies = q.fetch_all(db).await.strerr()?;
    for reply in loaded_replies {
        let Some(original_msgs) = replies_to_load.get(&reply.msg_id) else {
            continue;
        };
        for original_msg in original_msgs {
            replies.insert((*original_msg).clone(), reply.clone());
        }
    }
    Ok(replies)
}

async fn load_messages(
    db: &sqlx::Pool<sqlx::Sqlite>,
    reverse: bool,
    viewing: String,
    time: i64,
) -> Result<Vec<MsgData>, String> {
    if reverse {
        sqlx::query_as!(
        MsgData,
        "SELECT * FROM messages WHERE source = ? AND timestamp < ? ORDER BY timestamp DESC LIMIT ?",
        viewing,
        time,
        MSG_LOAD_LIMIT as i64
    )
        .fetch_all(db)
        .await
    } else {
        sqlx::query_as!(
        MsgData,
        "SELECT * FROM messages WHERE source = ? AND timestamp > ? ORDER BY timestamp ASC LIMIT ?",
        viewing,
        time,
        MSG_LOAD_LIMIT as i64
    )
        .fetch_all(db)
        .await
    }
    .strerr()
}

async fn load_reactions(
    db: &sqlx::Pool<sqlx::Sqlite>,
    messages: &Vec<MsgData>,
) -> Result<Vec<ReactionData>, String> {
    let mut query = String::from("SELECT * FROM reactions WHERE message_id IN ");
    query_append_list(&mut query, messages.len());

    let mut q = sqlx::query_as::<_, ReactionData>(&query);
    for id in messages {
        q = q.bind(&id.msg_id);
    }
    let reactions = q.fetch_all(db).await.strerr()?;
    Ok(reactions)
}

fn query_append_list(query: &mut String, len: usize) {
    if !query.ends_with(' ') {
        query.push(' ');
    }
    query.push('(');
    for i in 0..len {
        if i != 0 {
            query.push_str(", ");
        }
        query.push('?');
    }
    query.push(')');
}
