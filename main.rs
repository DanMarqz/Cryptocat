// Se importan las librerías y módulos necesarios
use teloxide::{prelude::*, utils::command::BotCommands};    // Librería para crear bots de Telegram
use rust_decimal::prelude::*;                               // para trabajar con números decimales
use serde::Deserialize;                                     // para deserializar (convertir) datos desde formato JSON
use dotenv;                                                 // para cargar variables de entorno desde un archivo .env
use pretty_env_logger;                                      // para gestionar el log con colores y formato bonito
use log;                                                    // para registrar mensajes en el log

// La función main es el punto de entrada del programa.
// La anotación #[tokio::main] indica que se ejecutará en el runtime asíncrono de Tokio
#[tokio::main]
async fn main() {
    // Carga las variables de entorno definidas en un archivo .env, si existe
    match dotenv::dotenv() {
        Ok(_) => { 
            println!("Archivo .env cargado correctamente."); 
        }
        Err(err) => { 
            println!("No se encontró archivo .env. Se usarán las variables de entorno del sistema. Error: {:?}", err); 
        }
    };

    // Inicializa el logger para mostrar mensajes de debug/información en consola
    pretty_env_logger::init();
    log::info!("Starting command bot...");

    // Crea una instancia del bot usando el token almacenado en las variables de entorno
    let bot = Bot::from_env();

    // Ejecuta el REPL (Read-Eval-Print Loop) para procesar comandos utilizando un closure
    // Al recibir un comando, se llama a la función `answer`
    Command::repl(bot, |bot, msg, cmd| async move {
        answer(bot, msg, cmd).await
    }).await;
}

// Se define una enumeración que representa los comandos que el bot soporta.
// La macro BotCommands junto con atributos facilitan la generación automática de ayuda y alias.
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "Display this text.")]
    Help,
    #[command(description = "Get USDT/BTC price.")]
    GetBtcPrice,
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
        Command::Help => {
            // Envía un mensaje de ayuda con la descripción de los comandos disponibles
            bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?
        }
        Command::GetBtcPrice => {
            // Llama a la función que obtiene el precio del bitcoin
            match get_bitcoin_price().await {
                Ok(val) => {
                    // Se formatea el precio a 2 decimales y se envía al usuario
                    let price = format!("{:.2}", val);
                    bot.send_message(msg.chat.id, format!("The price of the bitcoin is: {}", price)).await?
                }
                Err(err) => {
                    // En caso de error, se informa al usuario
                    bot.send_message(msg.chat.id, format!("Error fetching bitcoin price: {:?}", err)).await?
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