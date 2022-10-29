use anyhow::{Context as _, Result};
use deltachat::{
    chat::{self, Chat, ChatId},
    config::Config,
    constants::Chattype,
    context::Context,
    message::{Message, MsgId, Viewtype},
    stock_str::StockStrings,
    EventType, Events,
};
use log::{debug, error, info, warn};
use std::{env, sync::Arc};
use tokio::{
    signal,
    sync::mpsc::{self, Receiver},
};

use crate::{
    server::{Server, WebhookEvent},
    utils::configure_from_env,
};

struct State;

pub struct Bot {
    dc_ctx: Context,
    hook_receiver: Receiver<WebhookEvent>,
    hook_server: Server,
    state: Arc<State>,
}

impl Bot {
    pub async fn new() -> Self {
        let dbdir = env::current_dir().unwrap().join("deltachat-db");
        std::fs::create_dir_all(dbdir.clone())
            .context("failed to create db folder")
            .unwrap();
        let dbfile = dbdir.join("db.sqlite");
        let ctx = Context::new(dbfile.as_path(), 1, Events::new(), StockStrings::new())
            .await
            .context("Failed to create context")
            .unwrap();
        let is_configured = ctx.get_config_bool(Config::Configured).await.unwrap();
        if !is_configured {
            info!("configuring");
            configure_from_env(&ctx).await.unwrap();
            info!("configuration done");
        }

        let (tx, rx) = mpsc::channel(100);

        Self {
            dc_ctx: ctx,
            hook_receiver: rx,
            state: Arc::new(State),
            hook_server: Server::new(tx),
        }
    }

    pub async fn start(&self) {
        let events_emitter = self.dc_ctx.get_event_emitter();
        let ctx = self.dc_ctx.clone();
        tokio::spawn(async move {
            while let Some(event) = events_emitter.recv().await {
                Self::dc_event_handler(&ctx, event.typ).await;
            }
        });
        self.dc_ctx.start_io().await;
    }

    async fn dc_event_handler(ctx: &Context, event: EventType) {
        match event {
            EventType::ConfigureProgress { progress, comment } => {
                info!("Configuring progress: {progress} {comment:?}")
            }
            EventType::Info(msg) => info!("{msg}"),
            EventType::Warning(msg) => warn!("{msg}"),
            EventType::Error(msg) => error!("{msg}"),
            EventType::ConnectivityChanged => {
                warn!("ConnectivityChanged: {:?}", ctx.get_connectivity().await)
            }
            EventType::IncomingMsg { chat_id, msg_id } => {
                if let Err(err) = Self::handle_message(ctx, chat_id, msg_id).await {
                    error!("error handling message: {err}");
                }
            }
            other => {
                debug!("[unhandled event] {other:?}");
            }
        }
    }

    async fn handle_message(ctx: &Context, chat_id: ChatId, msg_id: MsgId) -> Result<()> {
        let chat = Chat::load_from_db(ctx, chat_id).await?;
        let msg = Message::load_from_db(ctx, msg_id).await?;

        info!(
            "recieved message '{}' in chat with type {:?}",
            msg.get_text().unwrap_or_default(),
            chat.get_type()
        );

        Ok(())
    }

    pub async fn stop(self) {
        self.dc_ctx.stop_io().await;
        self.hook_server.stop()
    }
}
