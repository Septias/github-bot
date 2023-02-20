//! Entry for the bot code

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
use itertools::Itertools;
use log::{debug, error, info, warn};
use std::{collections::HashMap, env, sync::Arc};
use tokio::sync::mpsc::{self, Receiver};

use crate::{
    db::{Repository, DB},
    parser::{Cli, Commands, Family},
    rest_api::{create_hook, get_repository, remove_hook},
    server::Server,
    shared::{issue::IssueEvent, pr::PREvent, Repository as SharedRepo, WebhookEvent},
    utils::{configure_from_env, send_text_to_all},
};

/// Internal representation of a git repository that can be subscribed to
#[derive(Debug, Default)]
pub struct GitRepository {
    pub name: String,
    pub id: RepositoryId,
}

type RepositoryId = i64;

/// Github Bot state
pub struct State {
    pub db: DB,
    pub ip: String,
}

/// Github Bot
pub struct Bot {
    dc_ctx: Context,
    hook_receiver: Option<Receiver<WebhookEvent>>,
    hook_server: Server,
    state: Arc<State>,
}

impl Bot {
    pub async fn new() -> Self {
        let dbdir = env::current_dir().unwrap().join("deltachat.db");
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

        let db = DB::new("file://bot.db").await;

        Self {
            dc_ctx: ctx,
            hook_receiver: Some(rx),
            state: Arc::new(State {
                db,
                ip: pnet::datalink::interfaces()
                    .iter()
                    .find(|e| e.is_up() && !e.is_loopback() && !e.ips.is_empty())
                    .expect("should have an ip")
                    .ips
                    .get(0)
                    .unwrap()
                    .ip()
                    .to_string(),
            }),
            hook_server: Server::new(tx),
        }
    }

    /// Start the bot which includes:
    /// - starting dc-message-receive loop
    /// - starting webhook-receive loop
    ///   - starting receiving server
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
        info!("initiated dc message handler (1/4)");

        self.dc_ctx.start_io().await;

        info!("initiated dc io (2/4)");

        // start webhook-server
        self.hook_server.start();

        info!("initiated webhook server (3/4)");

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
        info!("initiated webhook handler (4/4)");
        info!("successfully started bot! 🥳");
    }

    /// Handle _all_ dc-events
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

    /// Handles chat messages from clients
    async fn handle_dc_message(
        ctx: &Context,
        state: Arc<State>,
        chat_id: ChatId,
        msg_id: MsgId,
    ) -> Result<()> {
        let msg = Message::load_from_db(ctx, msg_id).await?;
        if let Some(text) = msg.get_text() {
            // only react to messages with right keywoard
            if text.starts_with("gh") {
                match <Cli as CommandFactory>::command().try_get_matches_from(text.split(' ')) {
                    Ok(mut matches) => {
                        let res = <Cli as FromArgMatches>::from_arg_matches_mut(&mut matches)?;

                        match &res.command {
                            Commands::Subscribe { .. } => {
                                info!("adding subscriber");
                                state.db.add_subscriber(res.command, chat_id).await;
                                send_text_msg(ctx, chat_id, "Added event listener".to_string())
                                    .await?;
                            }
                            Commands::Unsubscribe { .. } => {
                                info!("removing subscriber");
                                state.db.remove_subscriber(res.command, chat_id).await;
                                send_text_msg(ctx, chat_id, "Removed event listener".to_string())
                                    .await?;
                            }
                            Commands::Repositories { repo_subcommands } => match repo_subcommands {
                                crate::parser::RepoSubcommands::List => {
                                    let repos = state.db.get_repository_ids().await?;
                                    let text = if !repos.is_empty() {
                                        format!(
                                            "Available repositories:\n{}",
                                            repos.iter().join("\n")
                                        )
                                    } else {
                                        "No repositories have been added yet".to_string()
                                    };
                                    error!("{text}");
                                    send_text_msg(ctx, chat_id, text).await?;
                                }
                                crate::parser::RepoSubcommands::Add {
                                    owner,
                                    repository,
                                    api_key,
                                } => match create_hook(owner, repository, api_key, &state.ip).await
                                {
                                    Ok(hook_id) => {
                                        let SharedRepo { id, url, .. } =
                                            get_repository(owner, repository, api_key).await?;
                                        state
                                            .db
                                            .add_repository(Repository {
                                                name: repository,
                                                owner,
                                                hook_id,
                                                id,
                                                url: &url,
                                            })
                                            .await?;
                                        info!("Added new webhook for repository {repository}");
                                        send_text_msg(
                                            ctx,
                                            chat_id,
                                            "Successfully added webhook".to_string(),
                                        )
                                        .await?;
                                    }
                                    Err(err) => {
                                        error!("{err}");
                                        send_text_msg(ctx, chat_id, err.to_string()).await?;
                                    }
                                },
                                crate::parser::RepoSubcommands::Remove {
                                    repository,
                                    api_key,
                                } => {
                                    let hook_id = state.db.get_hook_id(*repository).await?;
                                    let owner = state.db.get_owner(*repository).await.unwrap();
                                    let repo = state.db.get_name(*repository).await.unwrap();
                                    match remove_hook(&owner, &repo, hook_id, api_key).await {
                                        Ok(_) => {
                                            info!("removed webhook for repo {repository}");
                                            send_text_msg(
                                                ctx,
                                                chat_id,
                                                "Successfully removed repository".to_string(),
                                            )
                                            .await?;
                                        }
                                        Err(err) => {
                                            error!("{err}");
                                            send_text_msg(ctx, chat_id, err.to_string()).await?;
                                        }
                                    }
                                }
                            },
                        }
                    }
                    Err(err) => {
                        send_text_msg(ctx, chat_id, err.to_string()).await.unwrap();
                    }
                };
            }
        }

        Ok(())
    }

    /// Handle a parsed webhook-event
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
                issue,
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
                send_text_to_all(
                    &subs,
                    &format!(
                        "User {} triggered event `{action}` on issue {}",
                        sender.login, issue.title
                    ),
                    ctx,
                )
                .await?;
            }
            WebhookEvent::PR(PREvent {
                action,
                sender,
                repository,
                pull_request: pr,
            }) => {
                let subs = state
                    .db
                    .get_subscribers(repository.id, Family::Pr { pr_action: action })
                    .await
                    .unwrap();
                send_text_to_all(
                    &subs,
                    &format!(
                        "User {} trigged event `{action}` on PR {}",
                        sender.login, pr.title
                    ),
                    ctx,
                )
                .await?;
            }
        };
        Ok(())
    }

    pub async fn stop(self) {
        self.dc_ctx.stop_io().await;
        self.hook_server.stop()
    }
}
