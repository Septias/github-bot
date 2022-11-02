mod bot;
mod db;
mod parser;
mod server;
mod shared;
mod utils;
mod rest_api;

use bot::Bot;
use tokio::signal;

#[tokio::main]
async fn main() {
    env_logger::init();
    let mut bot = Bot::new().await;
    bot.start().await;
    signal::ctrl_c().await.unwrap();
    bot.stop().await;
}
