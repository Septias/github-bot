use anyhow::{Context as _, Result};
use clap::{CommandFactory, FromArgMatches};
use deltachat::{
    chat::{Chat, ChatId},
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
    parser::Cli,
    server::Server,
    shared::{
        issue::{IssueAction, IssueEvent},
        WebhookEvent,
    },
    utils::configure_from_env,
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
        tokio::spawn(async move {
            while let Some(event) = thing_receiver.recv().await {
                Self::handle_webhook(state.clone(), event).await
            }
        });
    }

    async fn dc_event_handler(ctx: &Context, state: Arc<State>, event: EventType) {
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
                if let Err(err) = Self::handle_dc_message(ctx, state, chat_id, msg_id).await {
                    error!("error handling message: {err}");
                }
            }
            other => {
                debug!("[unhandled event] {other:?}");
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
            if text.chars().nth(0).unwrap() == '!' {
                debug!("handling user request {:?}", text);

                let mut matches = <Cli as CommandFactory>::command()
                    .get_matches_from(once("throwaway").chain(text[1..].split(' ')));
                let res = <Cli as FromArgMatches>::from_arg_matches_mut(&mut matches);

                match res {
                    Ok(cli) => state.db.add_subscriber(cli.command, chat_id).await,
                    Err(e) => error!("{e}"),
                }
            }
        }

        Ok(())
    }

    async fn handle_webhook(state: Arc<State>, event: WebhookEvent) {
        debug!("Handling webhook event {}", event.event_type());

        match event {
            WebhookEvent::Issue(IssueEvent {
                sender,
                action,
                repository: repo,
            }) => match action {
                IssueAction::Opened => {
                    format!("User {} opened new issue", sender.login);
                    let subs = state
                        .db
                        .get_subscribers(repo.id, crate::db::WebhookAction::Issue(action))
                        .await;
                }
                _ => (), // String::from("Event occured")
            },
            WebhookEvent::PR(pr_event) => todo!(),
        };
    }

    async fn send_msg_to_subscribers(chats: &[ChatId]) {}

    pub async fn stop(self) {
        self.dc_ctx.stop_io().await;
        self.hook_server.stop()
    }
}
