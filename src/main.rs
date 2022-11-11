use app_logger::AppLogger;
use dotenvy::dotenv;

mod app_discord;
mod app_logger;
mod model;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    AppLogger::init().unwrap();

    if let Err(why) = dotenv() {
        log::warn!("Failed to load .env file: {}", why);
    }

    app_discord::run_discord_app().await;
}
