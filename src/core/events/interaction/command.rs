use std::{mem, sync::Arc};

use eyre::Result;

use crate::{
    core::{
        commands::slash::{InteractionCommand, INTERACTION_COMMANDS},
        Context,
    },
    util::{ext::InteractionCommandExt, interaction::InteractionCommand as InteractionCommandBase},
};

pub async fn handle_command(ctx: Arc<Context>, mut command: InteractionCommandBase) {
    let name = mem::take(&mut command.data.name);

    let cmd = match INTERACTION_COMMANDS.command(&name) {
        Some(cmd) => cmd,
        None => return error!(?name, "Unknown interaction command"),
    };

    match process_command(ctx, command, cmd).await {
        Ok(_) => info!(?name, "Processed slash command"),
        Err(err) => error!(?name, ?err, "Failed to process command"),
    }
}

async fn process_command(
    ctx: Arc<Context>,
    command: InteractionCommandBase,
    cmd: InteractionCommand,
) -> Result<()> {
    match cmd {
        InteractionCommand::Chat(cmd) => {
            if cmd.flags.defer() {
                command.defer(&ctx, cmd.flags.ephemeral()).await?;
            }

            (cmd.exec)(ctx, command).await?;
        }
        InteractionCommand::Message(cmd) => (cmd.exec)(ctx, command).await?,
    }

    Ok(())
}
