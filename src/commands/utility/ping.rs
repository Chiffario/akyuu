use std::{sync::Arc, time::Instant};

use command_macros::SlashCommand;
use eyre::{ContextCompat, Result, WrapErr};
use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    core::Context,
    util::{
        builder::MessageBuilder,
        ext::{InteractionCommandExt, MessageExt},
        interaction::InteractionCommand,
    },
};

#[derive(CommandModel, CreateCommand, SlashCommand)]
#[command(name = "ping")]
#[flags(SKIP_DEFER)]
/// Check if the bot is online
pub struct Ping;

async fn slash_ping(ctx: Arc<Context>, command: InteractionCommand) -> Result<()> {
    let builder = MessageBuilder::new().content("Pong");
    let start = Instant::now();

    command
        .callback(&ctx, builder, false)
        .await
        .wrap_err("Failed to callback")?;

    let response_raw = ctx
        .interaction()
        .response(&command.token)
        .await
        .wrap_err("Failed to receive response")?;

    let elapsed = (Instant::now() - start).as_millis();

    let response = response_raw
        .model()
        .await
        .wrap_err("Failed to deserialize response")?;

    let content = format!(":ping_pong: Pong! ({elapsed}ms)");
    let builder = MessageBuilder::new().content(content);

    response
        .update(&ctx, &builder, command.permissions)
        .wrap_err("Lacking permission to update message")?
        .await?;

    Ok(())
}
