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
use std::env;
use tokio::signal;

async fn handle() -> anyhow::Result<()> {
    let dbdir = env::current_dir()?.join("deltachat-db");
    std::fs::create_dir_all(dbdir.clone()).context("failed to create data folder")?;
    let dbfile = dbdir.join("db.sqlite");
    let ctx = Context::new(dbfile.as_path(), 1, Events::new(), StockStrings::new())
        .await
        .context("Failed to create context")?;
    let info = ctx.get_info().await;

    let events_emitter = ctx.get_event_emitter();
    let emitter_ctx = ctx.clone();
    tokio::spawn(async move {
        while let Some(event) = events_emitter.recv().await {
            handle_event(&emitter_ctx, event.typ).await;
        }
    });

    let is_configured = ctx.get_config_bool(Config::Configured).await?;
    if !is_configured {
        println!("configuring");
        configure_from_env(&ctx).await?;
        println!("configuration done");
    }
    ctx.start_io().await;

    // wait for user interrupt using ctrc+c.
    signal::ctrl_c().await?;

    // Stop the deltachat tasks again.
    ctx.stop_io().await;

    Ok(())
}

async fn configure_from_env(ctx: &Context) -> Result<()> {
    let addr = env::var("addr")?;
    ctx.set_config(Config::Addr, Some(&addr)).await?;
    let pw = env::var("mail_pw")?;
    ctx.set_config(Config::MailPw, Some(&pw)).await?;
    ctx.set_config(Config::Bot, Some("1")).await?;
    ctx.set_config(Config::E2eeEnabled, Some("1")).await?;
    ctx.configure()
        .await
        .context("configure failed, you might have wrong credentials")?;

    Ok(())
}

/// Handles events emitted by the deltachat-core [`Context`].
///
/// Events are used for pretty much everything, this function shows handling some of the
/// more important ones:
///
/// - [`Info`], [`Warning`] and [`Error`] are the logging mechanism of deltachat-core, which
///   is always per-context.  Commonly these might be written to a logfile.
///
/// - [`IncomingMsg`] indicates a new message has arrived.
///
/// [`Info`]: EventType::Info
/// [`Warning`]: EventType::Warning
/// [`Error`]: EventType::Error
/// [`IncomingMsg`]: EventType::IncomingMsg
async fn handle_event(ctx: &Context, event: EventType) {
    match event {
        EventType::ConfigureProgress { progress, comment } => {
            println!("  progress: {progress} {comment:?}")
        }
        EventType::Info(msg) => println!(" I: {msg}"),
        EventType::Warning(msg) => println!(" W: {msg}"),
        EventType::Error(msg) => println!(" E: {msg}"),
        EventType::ConnectivityChanged => {
            println!("ConnectivityChanged: {:?}", ctx.get_connectivity().await)
        }
        EventType::IncomingMsg { chat_id, msg_id } => {
            if let Err(err) = handle_message(ctx, chat_id, msg_id).await {
                println!("error handling message: {err}");
            }
        }
        other => {
            println!("[unhandled event] {other:?}");
        }
    }
}

/// Handles a single incoming message.
///
/// Each message belongs to a chat, which is a conversation of messages between multiple
/// participants.
async fn handle_message(ctx: &Context, chat_id: ChatId, msg_id: MsgId) -> Result<()> {
    let chat = Chat::load_from_db(ctx, chat_id).await?;
    let msg = Message::load_from_db(ctx, msg_id).await?;

    println!(
        "recieved message '{}' in chat with type {:?}",
        msg.get_text().unwrap_or_default(),
        chat.get_type()
    );

    // Only respond to messages from a chat with only a single participant other than
    // ourselves.  This is also known as a "1:1" chat.
    if chat.get_type() == Chattype::Single {
        let mut message = Message::new(Viewtype::Text);
        message.set_text(msg.get_text());
        chat::send_msg(ctx, chat_id, &mut message).await?;
    }

    Ok(())
}
