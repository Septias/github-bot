pub mod bot;
pub mod db;
pub mod parser;
pub mod rest_api;
pub mod server;
pub mod shared;
pub mod utils;

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
