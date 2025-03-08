use teloxide::{prelude::*, utils::command::BotCommands};
use rust_decimal::prelude::*;
use serde::Deserialize;
use dotenv;
use pretty_env_logger;
use log;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok(); 
    pretty_env_logger::init();
    log::info!("Starting command bot...");

    let bot = Bot::from_env();

    // Ejecuta el repl de comandos usando un closure
    Command::repl(bot, |bot, msg, cmd| async move {
        answer(bot, msg, cmd).await
    }).await;
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "Display this text.")]
    Help,
    #[command(description = "Get USDT/BTC price.")]
    GetBtcPrice,
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct PriceResponse {
    price: String,
}

async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?
        }
        Command::GetBtcPrice => {
            match get_bitcoin_price().await {
                Ok(val) => {
                    let price = format!("{:.2}", val);
                    bot.send_message(msg.chat.id, format!("The price of the bitcoin is: {}", price)).await?
                }
                Err(err) => {
                    bot.send_message(msg.chat.id, format!("Error fetching bitcoin price: {:?}", err)).await?
                }
            }
        }
    };
    Ok(())
}

pub async fn get_bitcoin_price() -> Result<Decimal, Box<dyn std::error::Error + Send + Sync>> {
    let resp = reqwest::get("https://api.binance.com/api/v3/ticker/price?symbol=BTCUSDT").await?;
    let body = resp.json::<PriceResponse>().await?;
    let price = match Decimal::from_str(&body.price) {
        Ok(num) => num,
        Err(_) => {
            println!("Error on converting");
            Decimal::new(0, 1)
        }
    };
    Ok(price)
}