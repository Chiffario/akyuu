use std::sync::Arc;

use eyre::{Result, WrapErr};
use twilight_model::{
    application::command::{Command, CommandType},
    id::Id,
};

use crate::{
    active::{ActiveMessages, CreateIssue},
    core::{
        commands::slash::{CommandResult, MessageCommand},
        Context,
    },
    util::interaction::InteractionCommand,
    CONFIG,
};

pub static CREATE_ISSUE: MessageCommand = MessageCommand {
    create: create_command,
    exec: slash_create_issue,
    name: "Create github issue",
};

fn create_command() -> Command {
    Command {
        application_id: None,
        default_member_permissions: None,
        description: String::new(),
        description_localizations: None,
        dm_permission: Some(false),
        guild_id: Some(CONFIG.get().unwrap().discord_config.guild_id_as_marker()),
        id: None,
        kind: CommandType::Message,
        name: CREATE_ISSUE.name.to_owned(),
        name_localizations: None,
        nsfw: None,
        options: Vec::new(),
        version: Id::new(1),
        contexts: None,
        integration_types: None,
    }
}

fn slash_create_issue(ctx: Arc<Context>, command: InteractionCommand) -> CommandResult {
    Box::pin(slash_create_issue_(ctx, command))
}

async fn slash_create_issue_(ctx: Arc<Context>, command: InteractionCommand) -> Result<()> {
    let msg_id = command.data.target_id.expect("missing target_id").cast();

    let msg = ctx
        .http
        .message(command.channel_id, msg_id)
        .await
        .wrap_err("Failed to receive message of command")?
        .model()
        .await
        .wrap_err("Failed to deserialize message of command")?;

    let create_issue = CreateIssue::new(
        msg,
        CONFIG.get().unwrap().discord_config.guild_id_as_marker(),
    );

    ActiveMessages::builder(create_issue)
        .begin(ctx, command)
        .await
        .wrap_err("Failed to begin active message")
}
