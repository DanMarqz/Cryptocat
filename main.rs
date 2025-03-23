// Se importan las librerías y módulos necesarios
use rust_decimal::prelude::*;                               // para trabajar con números decimales
use serde::Deserialize;                                     // para deserializar (convertir) datos desde formato JSON
use dotenv;                                                 // para cargar variables de entorno desde un archivo .env
use pretty_env_logger;                                      // para gestionar el log con colores y formato bonito
use log;                                                    // para registrar mensajes en el log

// Importa los structs o enums para InlineKeyboardMarkup y InlineKeyboardButton
use teloxide::{prelude::*, utils::command::BotCommands};    // Librería para crear bots de Telegram
use teloxide::types::{WebAppInfo, InlineKeyboardMarkup, InlineKeyboardButton, Update, UpdateKind};
use teloxide::update_listeners::Polling;
use teloxide::update_listeners::AsUpdateStream;
use futures_util::stream::StreamExt;
use std::pin::Pin;

// La función main es el punto de entrada del programa.
// La anotación #[tokio::main] indica que se ejecutará en el runtime asíncrono de Tokio
#[tokio::main]
async fn main() {
    // Carga las variables de entorno
    match dotenv::dotenv() {
        Ok(_) => println!("Archivo .env cargado correctamente."),
        Err(err) => println!("No se encontró archivo .env. Se usarán las variables de entorno del sistema. Error: {:?}", err),
    };

    // Inicializa el logger
    pretty_env_logger::init();
    log::info!("Starting command bot...");

    // Crea una instancia del bot usando el token almacenado en las variables de entorno
    let bot = Bot::from_env();

    // Listener para procesar los comandos
    let bot_commands = bot.clone();
    let commands_fut = Command::repl(bot_commands, |bot, msg, cmd| async move {
        answer(bot, msg, cmd).await
    });

    // Listener para callback queries usando un update listener que es un Stream
    let bot_callbacks = bot.clone();
    let cb_fut = async move {
        let mut polling = Polling::builder(bot_callbacks.clone())
            .drop_pending_updates()
            .timeout(std::time::Duration::from_secs(30))
            .build();

        let mut stream = Box::pin(polling.as_stream());
        
        while let Some(update_result) = stream.next().await {
            if let Ok(update) = update_result {
                if let Update { kind: UpdateKind::CallbackQuery(query), .. } = update {
                    if let Err(err) = handle_callback_query(bot_callbacks.clone(), query).await {
                        log::error!("Error in callback query handler: {:?}", err);
                    }
                }
            }
        }
    };

    tokio::join!(commands_fut, cb_fut);
}

// Se define una enumeración que representa los comandos que el bot soporta.
// La macro BotCommands junto con atributos facilitan la generación automática de ayuda y alias.
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "About this bot.")]
    Info,
    #[command(description = "Display this text.")]
    Help,
    #[command(description = "Get USDT/BTC price.")]
    GetBtcPrice,
}

pub enum MenuButton {
    Commands,
    WebApp {
        text: String,
        web_app: WebAppInfo,
    },
    Default,
}

// Se define una estructura para deserializar la respuesta del API.
// El atributo Deserialize permite transformar el JSON recibido en una instancia de esta estructura.
#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct PriceResponse {
    price: String, // Aquí se espera que el JSON tenga una propiedad "price" que es un String
}

// Esta función procesa el comando recibido y envía la respuesta al usuario
async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    match cmd { // Se evalúa qué comando fue recibido
        Command::Info => {
            // Envía un mensaje con la info del bot tomando las variables de entorno APP_NAME y APP_VERSION
            bot.send_message(
                msg.chat.id,
                format!("Meow! Soy {}, en mi Version: {}. Solo puedo obtener el precio del Bitcoin por ahora. (BTC/USDT)",
                    std::env::var("APP_NAME").unwrap_or("Bot".to_string()),
                    std::env::var("APP_VERSION").unwrap_or("0.1".to_string())
                ))
            .await?
        }
        Command::Help => {
            // Envía un mensaje de ayuda con la descripción de los comandos disponibles
            bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?
        }
        Command::GetBtcPrice => {
            // Define un botón con callback data "update_btc_price"
            let keyboard = InlineKeyboardMarkup::default()
                .append_row(vec![
                    InlineKeyboardButton::callback("Update Price", "update_btc_price".to_string()),
                ]);
        
            match get_bitcoin_price().await {
                Ok(val) => {
                    let price = format!("{:.2}", val);
                    // Envía el mensaje inicial con el precio y el teclado adjunto
                    bot.send_message(
                        msg.chat.id, 
                        format!("The price of the bitcoin is: {}", price)
                    )
                    .reply_markup(keyboard)
                    .await?
                }
                Err(err) => {
                    bot.send_message(
                        msg.chat.id, 
                        format!("Error fetching bitcoin price: {:?}", err)
                    ).await?
                }
            }
        }
    };
    Ok(())
}

// Esta función se encarga de obtener el precio del bitcoin desde el API de Binance.
// Utiliza reqwest para hacer una petición HTTP asíncrona.
pub async fn get_bitcoin_price() -> Result<Decimal, Box<dyn std::error::Error + Send + Sync>> {
    // Hace una petición GET al API de Binance
    let resp = reqwest::get("https://api.binance.com/api/v3/ticker/price?symbol=BTCUSDT").await?;
    // Deserializa la respuesta JSON en la estructura PriceResponse
    let body = resp.json::<PriceResponse>().await?;
    // Intenta convertir el precio (String) a un tipo Decimal para manejo numérico
    let price = match Decimal::from_str(&body.price) {
        Ok(num) => num,
        Err(_) => {
            println!("Error on converting");
            // En caso de error al convertir, retorna un valor por defecto
            Decimal::new(0, 1)
        }
    };
    Ok(price)
}

async fn handle_callback_query(bot: Bot, query: CallbackQuery) -> ResponseResult<()> {
    if let Some(data) = &query.data {
        if data == "update_btc_price" {
            if let Some(message) = query.message {
                // Clona el id de la callback para poder reutilizarlo
                let callback_id = query.id.clone();
                // Obtiene el precio actualizado
                match get_bitcoin_price().await {
                    Ok(val) => {
                        let price = format!("{:.2}", val);
                        // Edita el mensaje para actualizar el precio
                        bot.edit_message_text(message.chat().id, message.id(), format!("The price of the bitcoin is: {}", price))
                            .await?;
                    }
                    Err(err) => {
                        // En caso de error, responde a la callback query
                        bot.answer_callback_query(query.id.clone())
                           .text(format!("Error fetching bitcoin price: {:?}", err))
                           .await?;
                    }
                }
                // Confirma la recepción de la callback query
                bot.answer_callback_query(callback_id).await?;
            }
        }
    }
    Ok(())
}