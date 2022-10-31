#![allow(unused)]
mod bot;
mod db;
mod parser;
mod server;
pub mod shared;
mod utils;

use bot::Bot;
use clap::Parser;
use parser::Cli;
use tokio::signal;

#[tokio::main]
async fn main() {
    env_logger::init();
    let mut bot = Bot::new().await;
    bot.start().await;
    signal::ctrl_c().await.unwrap();
    bot.stop().await;
}
