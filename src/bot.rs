use crate::price_fetcher::PriceFetchService;
use solana_program::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;
use teloxide::prelude::{Message, Requester};
use teloxide::Bot;
use teloxide::repls::CommandReplExt;
use teloxide::utils::command::BotCommands;

/// These commands are supported:
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    #[command(
        description = "Average pool price over the last few minutes. Usage example: /average <pool address in base64 encoding>"
    )]
    Average(String),
    #[command(
        description = "Last actual price. Usage example: /current <pool address in base64 encoding>"
    )]
    Current(String),
    #[command(description = "Show supported pools addresses")]
    Supported,
}


pub async fn setup_bot(price_fetch_service: Arc<PriceFetchService>) -> anyhow::Result<()> {
    const ERR_MSG: &str = "Incorrect input, please specify pool address after command";

    let bot = Bot::new("8129950231:AAFOB04snHB5J-5AIzMH8RUB1qtIH0Is_zY");

    bot.set_my_commands(Command::bot_commands()).await.expect("Failed to set commands.");

    Command::repl(bot, move |bot: Bot, msg: Message, cmd: Command| {
        let price_fetch_service = price_fetch_service.clone();
        async move {
            match cmd {
                Command::Average(pool_address) => {
                    let Ok(pool_address) = Pubkey::from_str(&pool_address) else {
                        bot.send_message(msg.chat.id, ERR_MSG.to_string()).await?;
                        return Ok(())
                    };

                    let average_resp = price_fetch_service.average(&pool_address).await;
                    bot.send_message(msg.chat.id, format!("{average_resp}")).await?;
                }
                Command::Current(pool_address) => {
                    let Ok(pool_address) = Pubkey::from_str(&pool_address) else {
                        bot.send_message(msg.chat.id, ERR_MSG.to_string()).await?;
                        return Ok(())
                    };

                    let current_resp = price_fetch_service.current(&pool_address).await;
                    bot.send_message(msg.chat.id, format!("{current_resp}")).await?;
                }
                Command::Supported => {
                    let supported_pools = price_fetch_service.supported_pools().join("\\n");
                    bot.send_message(msg.chat.id, supported_pools).await?;
                }
            }

            return Ok(())
        }
    }).await;

    Ok(())
}