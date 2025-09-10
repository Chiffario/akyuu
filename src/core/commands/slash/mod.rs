use std::pin::Pin;

use eyre::{Context, Result};
use futures::Future;
use radix_trie::{Trie, TrieCommon};
use twilight_http::client::InteractionClient;
use twilight_interactions::command::{ApplicationCommandData, CreateCommand};
use twilight_model::{
    application::command::Command,
    id::{marker::GuildMarker, Id},
};

use crate::commands::{github::*, utility::*};

pub use self::command::*;

mod command;

pub struct InteractionCommands(Trie<&'static str, InteractionCommand>);

pub type CommandResult = Pin<Box<dyn Future<Output = Result<()>> + 'static + Send>>;

impl InteractionCommands {
    pub fn command(&self, command: &str) -> Option<InteractionCommand> {
        self.0.get(command).copied()
    }

    pub async fn register(
        &self,
        client: &InteractionClient<'_>,
        guild: Id<GuildMarker>,
    ) -> Result<()> {
        info!("Creating {} interaction commands...", self.0.len());

        for cmd in self.0.values() {
            match cmd {
                InteractionCommand::Chat(cmd) => {
                    let cmd = (cmd.create)();
                    let name = cmd.name.clone();

                    Self::register_slash_command(cmd, client, guild)
                        .await
                        .wrap_err_with(|| format!("Failed to register slash command `{name}`"))?
                }
                InteractionCommand::Message(cmd) => {
                    let cmd = (cmd.create)();
                    let name = cmd.name.clone();

                    Self::register_message_command(cmd, client, guild)
                        .await
                        .wrap_err_with(|| format!("Failed to register message command `{name}`"))?
                }
            }
        }

        Ok(())
    }

    async fn register_slash_command(
        cmd: ApplicationCommandData,
        client: &InteractionClient<'_>,
        guild: Id<GuildMarker>,
    ) -> Result<()> {
        let mut builder = client
            .create_guild_command(guild)
            .chat_input(&cmd.name, &cmd.description)
            .command_options(&cmd.options);

        if let Some(ref localizations) = cmd.name_localizations {
            builder = builder.name_localizations(localizations);
        }

        if let Some(ref localizations) = cmd.description_localizations {
            builder = builder.description_localizations(localizations);
        }

        if let Some(default) = cmd.default_member_permissions {
            builder = builder.default_member_permissions(default);
        }

        if let Some(nsfw) = cmd.nsfw {
            builder = builder.nsfw(nsfw);
        }

        builder.await.wrap_err("Failed to create command")?;

        Ok(())
    }

    async fn register_message_command(
        cmd: Command,
        client: &InteractionClient<'_>,
        guild: Id<GuildMarker>,
    ) -> Result<()> {
        let mut builder = client.create_guild_command(guild).message(&cmd.name);

        if let Some(ref localizations) = cmd.name_localizations {
            builder = builder.name_localizations(localizations);
        }

        if let Some(default) = cmd.default_member_permissions {
            builder = builder.default_member_permissions(default);
        }

        if let Some(nsfw) = cmd.nsfw {
            builder = builder.nsfw(nsfw);
        }

        builder.await.wrap_err("Failed to create command")?;

        Ok(())
    }
}

macro_rules! slash_trie {
    ($(chat: $chat_cmd:ident => $chat_fun:ident,)*$(msg: $msg_cmd:ident,)*) => {
        let mut trie = Trie::new();

        $(trie.insert($chat_cmd::NAME, InteractionCommand::Chat(&$chat_fun));)*
        $(trie.insert($msg_cmd.name, InteractionCommand::Message(&$msg_cmd));)*

        InteractionCommands(trie)
    }
}

lazy_static::lazy_static! {
    pub static ref INTERACTION_COMMANDS: InteractionCommands = {
        slash_trie! {
            chat: Ping => PING_SLASH,
            msg: CREATE_ISSUE,
        }
    };
}
