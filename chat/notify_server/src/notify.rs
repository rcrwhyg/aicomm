use std::{collections::HashSet, sync::Arc};

use crate::AppState;
use anyhow::Result;
use chat_core::{Chat, Message};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgListener;
use tracing::{info, warn};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "event")]
pub enum AppEvent {
    NewChat(Chat),
    AddToChat(Chat),
    RemoveFromChat(Chat),
    NewMessage(Message),
}

#[derive(Debug)]
struct Notification {
    // users being impacted, so we should send the notification to them
    user_ids: HashSet<u64>,
    event: Arc<AppEvent>,
}

// pg_notify('chat_updated', json_build_object('op', TG_OP, 'old', OLD, 'new', NEW)::text);
#[derive(Debug, Serialize, Deserialize)]
struct ChatUpdated {
    op: String,
    old: Option<Chat>,
    new: Option<Chat>,
}

// pg_notify('chat_message_created', row_to_json(NEW)::text);
#[derive(Debug, Serialize, Deserialize)]
struct ChatMessageCreated {
    message: Message,
    members: Vec<u64>,
}

pub async fn setup_pg_listener(state: AppState) -> Result<()> {
    let mut listener = PgListener::connect(&state.config.server.db_url).await?;
    listener.listen("chat_updated").await?;
    listener.listen("chat_message_created").await?;

    let mut stream = listener.into_stream();

    tokio::spawn(async move {
        while let Some(Ok(notify)) = stream.next().await {
            info!("Received notification: {:?}", notify);
            let notification = Notification::load(notify.channel(), notify.payload())?;
            let users = &state.users;
            for user_id in notification.user_ids {
                if let Some(tx) = users.get(&user_id) {
                    info!("Sending notification to user[{}]", user_id);
                    if let Err(e) = tx.send(notification.event.clone()) {
                        warn!("Failed to send notification to user[{}]: {}", user_id, e);
                    }
                }
            }
        }
        Ok::<_, anyhow::Error>(())
    });

    Ok(())
}

impl Notification {
    fn load(r#type: &str, payload: &str) -> Result<Self> {
        match r#type {
            "chat_updated" => {
                let payload = serde_json::from_str::<ChatUpdated>(payload)?;
                info!("Chat updated: {:?}", payload);
                let user_ids =
                    get_affected_chat_user_ids(payload.old.as_ref(), payload.new.as_ref());
                let event = match payload.op.as_str() {
                    "INSERT" => AppEvent::NewChat(payload.new.expect("new should exist")),
                    "UPDATE" => AppEvent::AddToChat(payload.old.expect("new should exist")),
                    "DELETE" => AppEvent::RemoveFromChat(payload.old.expect("old should exist")),
                    _ => return Err(anyhow::anyhow!("Invalid operation")),
                };
                Ok(Self {
                    user_ids,
                    event: Arc::new(event),
                })
            }
            "chat_message_created" => {
                let payload = serde_json::from_str::<ChatMessageCreated>(payload)?;
                let user_ids = payload.members.iter().copied().collect();
                Ok(Self {
                    user_ids,
                    event: Arc::new(AppEvent::NewMessage(payload.message)),
                })
            }
            _ => Err(anyhow::anyhow!("Invalid notification type")),
        }
    }
}

fn get_affected_chat_user_ids(old: Option<&Chat>, new: Option<&Chat>) -> HashSet<u64> {
    match (old, new) {
        (Some(old), Some(new)) => {
            // diff old/new members, if identical, no need to notify, otherwise notify the union of both
            let old_members: HashSet<_> = old.members.iter().map(|v| *v as u64).collect();
            let new_members: HashSet<_> = new.members.iter().map(|v| *v as u64).collect();

            if old_members == new_members {
                HashSet::new()
            } else {
                old_members.union(&new_members).copied().collect()
            }
        }
        // (Some(chat), None) | (None, Some(chat)) => chat.user_ids.clone(),
        (Some(old), None) => old.members.iter().map(|v| *v as u64).collect(),
        (None, Some(new)) => new.members.iter().map(|v| *v as u64).collect(),
        _ => HashSet::new(),
    }
}
