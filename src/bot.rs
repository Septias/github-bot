use anyhow::{Context as _, Result};
use clap::{CommandFactory, FromArgMatches};
use deltachat::{
    chat::{send_text_msg, ChatId},
    config::Config,
    context::Context,
    message::{Message, MsgId},
    stock_str::StockStrings,
    EventType, Events,
};
use log::{debug, error, info, warn};
use std::{collections::HashMap, env, iter::once, sync::Arc};
use tokio::sync::mpsc::{self, Receiver};

use crate::{
    db::DB,
    parser::{Cli, Commands, Family},
    server::Server,
    shared::{issue::IssueEvent, pr::PREvent, WebhookEvent},
    utils::{configure_from_env, send_text_to_all},
};

#[derive(Debug, Default)]
pub struct GitRepository {
    pub name: String,
    pub id: RepositoryId,
}

type RepositoryId = String;

pub struct State {
    pub repos: HashMap<RepositoryId, GitRepository>,
    pub db: DB,
}

pub struct Bot {
    dc_ctx: Context,
    hook_receiver: Option<Receiver<WebhookEvent>>,
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

        let repositories = [GitRepository {
            name: "test".to_owned(),
            id: "".to_owned(),
        }];

        let db = DB::new().await;
        db.init().await;

        Self {
            dc_ctx: ctx,
            hook_receiver: Some(rx),
            state: Arc::new(State {
                repos: repositories
                    .into_iter()
                    .map(|rep| (rep.id.clone(), rep))
                    .collect(),
                db,
            }),
            hook_server: Server::new(tx),
        }
    }

    pub async fn start(&mut self) {
        // start dc message handler
        let events_emitter = self.dc_ctx.get_event_emitter();
        let ctx = self.dc_ctx.clone();
        let state = self.state.clone();
        tokio::spawn(async move {
            while let Some(event) = events_emitter.recv().await {
                Self::dc_event_handler(&ctx, state.clone(), event.typ).await;
            }
        });
        self.dc_ctx.start_io().await;

        // start webhook-server
        self.hook_server.start();

        // start webhook-handler
        let mut thing_receiver = self.hook_receiver.take().unwrap();
        let state = self.state.clone();
        let ctx = self.dc_ctx.clone();
        tokio::spawn(async move {
            while let Some(event) = thing_receiver.recv().await {
                if let Err(e) = Self::handle_webhook(state.clone(), &ctx, event).await {
                    error!("{e}")
                }
            }
        });
    }

    async fn dc_event_handler(ctx: &Context, state: Arc<State>, event: EventType) {
        match event {
            EventType::ConfigureProgress { progress, comment } => {
                info!("DC: Configuring progress: {progress} {comment:?}")
            }
            EventType::Info(..) => (), //info!("DC: {msg}"),
            EventType::Warning(msg) => warn!("DC: {msg}"),
            EventType::Error(msg) => error!("DC: {msg}"),
            EventType::ConnectivityChanged => {
                warn!(
                    "DC: ConnectivityChanged: {:?}",
                    ctx.get_connectivity().await
                )
            }
            EventType::IncomingMsg { chat_id, msg_id } => {
                if let Err(err) = Self::handle_dc_message(ctx, state, chat_id, msg_id).await {
                    error!("DC: error handling message: {err}");
                }
            }
            other => {
                debug!("DC: [unhandled event] {other:?}");
            }
        }
    }

    async fn handle_dc_message(
        ctx: &Context,
        state: Arc<State>,
        chat_id: ChatId,
        msg_id: MsgId,
    ) -> Result<()> {
        let msg = Message::load_from_db(ctx, msg_id).await?;
        if let Some(text) = msg.get_text() {
            if text.starts_with('!') {
                match <Cli as CommandFactory>::command()
                    .try_get_matches_from(once("throwaway").chain(text[1..].split(' ')))
                {
                    Ok(mut matches) => {
                        let res = <Cli as FromArgMatches>::from_arg_matches_mut(&mut matches)?;
                        if matches!(res.command, Commands::Subscribe { .. }) {
                            info!("adding subscriber");
                            state.db.add_subscriber(res.command, chat_id).await
                        } else {
                            info!("removing subscriber");
                            state.db.remove_subscriber(res.command, chat_id).await
                        };
                    }
                    Err(err) => drop(send_text_msg(ctx, chat_id, err.to_string()).await.unwrap()),
                };
            }
        }

        Ok(())
    }

    async fn handle_webhook(
        state: Arc<State>,
        ctx: &Context,
        event: WebhookEvent,
    ) -> anyhow::Result<()> {
        info!("Handling webhook event {}", event);
        match event {
            WebhookEvent::Issue(IssueEvent {
                sender,
                action,
                repository,
            }) => {
                let subs = state
                    .db
                    .get_subscribers(
                        repository.id,
                        Family::Issue {
                            issue_action: action,
                        },
                    )
                    .await
                    .unwrap();
                send_text_to_all(&subs, &format!("User {} {action} issue", sender.login), ctx)
                    .await?;
            }
            WebhookEvent::PR(PREvent {
                action,
                sender,
                repository,
            }) => {
                let subs = state
                    .db
                    .get_subscribers(repository.id, Family::PR { pr_action: action })
                    .await
                    .unwrap();
                send_text_to_all(&subs, &format!("User {} {action} PR", sender.login), ctx).await?;
            }
        };
        Ok(())
    }

    async fn _send_msg_to_subscribers(_chats: &[ChatId]) {}

    pub async fn stop(self) {
        self.dc_ctx.stop_io().await;
        self.hook_server.stop()
    }
}
