#![allow(unused)]
mod bot;
mod parser;
mod server;
mod utils;
use bot::Bot;
use parser::test;
use tokio::signal;

#[tokio::main]
async fn main() {
    let bot = Bot::new().await;
    bot.start().await;
    signal::ctrl_c().await.unwrap();
    bot.stop().await;
}
