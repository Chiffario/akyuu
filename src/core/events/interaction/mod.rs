use std::sync::Arc;

use twilight_model::application::interaction::{Interaction, InteractionData, InteractionType};

use crate::{
    active::ActiveMessages,
    core::Context,
    util::interaction::{InteractionCommand, InteractionComponent, InteractionModal},
};

use self::{autocomplete::handle_autocomplete, command::handle_command};

mod autocomplete;
mod command;

pub async fn handle_interaction(ctx: Arc<Context>, interaction: Interaction) {
    let Interaction {
        app_permissions: permissions,
        channel_id,
        data,
        guild_id,
        id,
        kind,
        member,
        message,
        token,
        user,
        ..
    } = interaction;

    {
        let user_opt = member
            .as_ref()
            .and_then(|member| member.user.as_ref())
            .or(user.as_ref());

        let Some(user) = user_opt else {
            warn!("Received interaction without user; ignoring");

            return;
        };

        if !(&ctx.config.discord_config.operator_id_as_marker() == &user.id) {
            let name = &user.name;
            info!("User `{name}` attempted to use interaction but is not listed as operator");

            return;
        }
    }

    let Some(channel_id) = channel_id else {
        return warn!(?kind, "No channel id for interaction kind");
    };

    match data {
        Some(InteractionData::ApplicationCommand(data)) => {
            let cmd = InteractionCommand {
                permissions,
                channel_id,
                data,
                guild_id,
                id,
                member,
                token,
                user,
            };

            match kind {
                InteractionType::ApplicationCommand => handle_command(ctx, cmd).await,
                InteractionType::ApplicationCommandAutocomplete => {
                    handle_autocomplete(ctx, cmd).await
                }
                _ => warn!(?kind, "Got unexpected interaction kind"),
            }
        }
        Some(InteractionData::MessageComponent(data)) => {
            let Some(message) = message else {
                return warn!("No message in interaction component");
            };

            let component = InteractionComponent {
                permissions,
                channel_id,
                data,
                guild_id,
                id,
                member,
                message,
                token,
                user,
            };

            ActiveMessages::handle_component(&ctx, component).await
        }
        Some(InteractionData::ModalSubmit(data)) => {
            let modal = InteractionModal {
                permissions,
                channel_id,
                data,
                guild_id,
                id,
                member,
                message,
                token,
                user,
            };

            ActiveMessages::handle_modal(&ctx, modal).await
        }
        _ => {}
    }
}
