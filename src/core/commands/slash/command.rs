use std::sync::Arc;

use twilight_interactions::command::ApplicationCommandData;
use twilight_model::application::command::Command;

use crate::{
    core::{commands::flags::CommandFlags, Context},
    util::interaction::InteractionCommand as InteractionCommandBase,
};

use super::CommandResult;

#[derive(Copy, Clone)]
pub enum InteractionCommand {
    Chat(&'static SlashCommand),
    Message(&'static MessageCommand),
}

pub struct SlashCommand {
    pub create: fn() -> ApplicationCommandData,
    pub exec: fn(Arc<Context>, InteractionCommandBase) -> CommandResult,
    pub flags: CommandFlags,
}

pub struct MessageCommand {
    pub create: fn() -> Command,
    pub exec: fn(Arc<Context>, InteractionCommandBase) -> CommandResult,
    pub name: &'static str,
}
