use crate::price_fetcher::{PriceFetchResponseType, PriceFetchService};
use solana_program::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;
use teloxide::prelude::{Message, Requester};
use teloxide::Bot;
use teloxide::repls::CommandReplExt;
use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    #[command(description = "Average pool price over the last few minutes.")]
    Average,
    #[command(description = "Last actual price.")]
    Current,
}


pub async fn setup_bot(price_fetch_service: Arc<PriceFetchService>) -> anyhow::Result<()> {
    let bot = Bot::from_env();

    bot.set_my_commands(Command::bot_commands()).await.expect("Failed to set commands.");

    // TODO: Provide multiple pools support
    let pool_address = Pubkey::from_str("EP2ib6dYdEeqD8MfE2ezHCxX3kP3K2eLKkirfPm5eyMx").unwrap();

    Command::repl(bot, move |bot: Bot, msg: Message, cmd: Command| {
        let price_fetch_service = price_fetch_service.clone();
        async move {
            match cmd {
                Command::Average => {
                    let average_resp = price_fetch_service.average(&pool_address).await;

                    if let PriceFetchResponseType::NoDataFound  = average_resp.response_type {
                        let current_resp = price_fetch_service.current(&pool_address).await;
                        let response = format!("No data found for last 5 minutes. {current_resp}");
                        bot.send_message(msg.chat.id, response).await?;
                    }

                    bot.send_message(msg.chat.id, format!("{average_resp}")).await?;
                }
                Command::Current => {
                    let current_resp = price_fetch_service.current(&pool_address).await;
                    bot.send_message(msg.chat.id, format!("{current_resp}")).await?;
                }
            }

            return Ok(())
        }
    }).await;

    Ok(())
}