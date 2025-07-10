use std::{sync::Arc, time::Duration};

use enum_dispatch::enum_dispatch;
use eyre::{Report, Result, WrapErr};
use flexmap::tokio::TokioMutexMap;
use futures::future::{self, BoxFuture};
use tokio::{
    sync::watch::{self, Receiver, Sender},
    time::sleep,
};
use twilight_model::{
    channel::message::Component,
    id::{
        marker::{ChannelMarker, MessageMarker},
        Id,
    },
};

mod create_issue;

use crate::{
    core::Context,
    util::{
        builder::{EmbedBuilder, MessageBuilder, ModalBuilder},
        ext::{ComponentExt, InteractionCommandExt, MessageExt, ModalExt},
        interaction::{InteractionCommand, InteractionComponent, InteractionModal},
    },
};

pub use self::create_issue::*;

pub struct ActiveMessagesBuilder {
    inner: ActiveMessage,
    start_by_update: Option<bool>,
}

impl ActiveMessagesBuilder {
    pub fn new(active_msg: impl Into<ActiveMessage>) -> Self {
        Self {
            inner: active_msg.into(),
            start_by_update: None,
        }
    }

    pub async fn begin(self, ctx: Arc<Context>, orig: InteractionCommand) -> Result<()> {
        let Self {
            inner: mut active_msg,
            start_by_update,
        } = self;

        let embed = active_msg
            .build_page(&ctx)
            .await
            .wrap_err("Failed to build page")?;

        let components = active_msg.build_components();
        let builder = MessageBuilder::new().embed(embed).components(components);

        let response_raw = if start_by_update.unwrap_or(false) {
            orig.update(&ctx, &builder)
                .await
                .wrap_err("Failed to update")?
        } else {
            orig.callback(&ctx, builder, false)
                .await
                .wrap_err("Failed to callback")?;

            ctx.interaction()
                .response(&orig.token)
                .await
                .wrap_err("Failed to get response message")?
        };

        let response = response_raw
            .model()
            .await
            .wrap_err("Failed to deserialize response")?;

        let channel = response.channel_id;
        let msg = response.id;
        let (tx, rx) = watch::channel(());

        Self::spawn_timeout(Arc::clone(&ctx), rx, msg, channel);

        let full = FullActiveMessage { active_msg, tx };
        ctx.active_msgs.insert(msg, full).await;

        Ok(())
    }

    #[allow(unused)]
    pub fn start_by_update(self, start_by_update: bool) -> Self {
        Self {
            start_by_update: Some(start_by_update),
            ..self
        }
    }

    fn spawn_timeout(
        ctx: Arc<Context>,
        mut rx: Receiver<()>,
        msg: Id<MessageMarker>,
        channel: Id<ChannelMarker>,
    ) {
        const MINUTE: Duration = Duration::from_secs(60);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    res = rx.changed() => if res.is_ok() {
                        continue
                    } else {
                        return
                    },
                    _ = sleep(MINUTE) => {
                        let pagination_active = ctx.active_msgs.remove(msg).await.is_some();

                        if pagination_active  {
                            let builder = MessageBuilder::new().components(Vec::new());

                            if let Some(update_fut) = (msg, channel).update(&ctx, &builder, None) {
                                if let Err(err) = update_fut.await {
                                    warn!(?err, "Failed to remove components");
                                }
                            }
                        }

                        return;
                    },
                }
            }
        });
    }
}

#[derive(Default)]
pub struct ActiveMessages {
    inner: TokioMutexMap<Id<MessageMarker>, FullActiveMessage>,
}

impl ActiveMessages {
    pub fn builder(active_msg: impl Into<ActiveMessage>) -> ActiveMessagesBuilder {
        ActiveMessagesBuilder::new(active_msg)
    }

    pub async fn handle_component(ctx: &Context, mut component: InteractionComponent) {
        let msg_id = component.message.id;
        let mut guard = ctx.active_msgs.inner.lock(&msg_id).await;

        let Some(FullActiveMessage { active_msg, tx }) = guard.get_mut() else {
            return error!(
                name = component.data.custom_id,
                ?component,
                "Unknown message component",
            );
        };

        match active_msg.handle_component(&mut component).await {
            ComponentResult::CreateModal(modal) => {
                if let Err(err) = component.modal(ctx, modal).await {
                    return error!(?err, "Failed to create modal");
                }

                let _ = tx.send(());
            }
            ComponentResult::BuildPage => match active_msg.build_page(ctx).await {
                Ok(embed) => {
                    let builder = MessageBuilder::new()
                        .embed(embed)
                        .components(active_msg.build_components());

                    if let Err(err) = component.callback(ctx, builder).await {
                        return error!(
                            name = component.data.custom_id,
                            ?err,
                            "Failed to callback component",
                        );
                    }

                    let _ = tx.send(());
                }
                Err(err) => error!(
                    name = component.data.custom_id,
                    ?err,
                    "Failed to build page for component",
                ),
            },
            ComponentResult::Err(err) => {
                error!(
                    name = component.data.custom_id,
                    ?err,
                    "Failed to process component",
                )
            }
        }
    }

    pub async fn handle_modal(ctx: &Context, mut modal: InteractionModal) {
        let mut guard = match modal.message {
            Some(ref msg) => ctx.active_msgs.inner.own(msg.id).await,
            None => return warn!("Received modal without message"),
        };

        let Some(FullActiveMessage { active_msg, tx }) = guard.get_mut() else {
            return error!(name = modal.data.custom_id, ?modal, "Unknown modal");
        };

        if let Err(err) = active_msg.handle_modal(&mut modal).await {
            return error!(name = modal.data.custom_id, ?err, "Failed to process modal");
        }

        match active_msg.build_page(ctx).await {
            Ok(embed) => {
                let builder = MessageBuilder::new()
                    .embed(embed)
                    .components(active_msg.build_components());

                if let Err(err) = modal.callback(ctx, builder).await {
                    return error!(
                        name = modal.data.custom_id,
                        ?err,
                        "Failed to callback modal",
                    );
                }

                let _ = tx.send(());
            }
            Err(err) => error!(
                name = modal.data.custom_id,
                ?err,
                "Failed to build page for modal",
            ),
        }
    }

    async fn remove(&self, msg: Id<MessageMarker>) -> Option<FullActiveMessage> {
        self.inner.lock(&msg).await.remove()
    }

    async fn insert(&self, msg: Id<MessageMarker>, active_msg: FullActiveMessage) {
        self.inner.own(msg).await.insert(active_msg);
    }
}

struct FullActiveMessage {
    active_msg: ActiveMessage,
    tx: Sender<()>,
}

#[enum_dispatch(IActiveMessage)]
pub enum ActiveMessage {
    CreateIssue,
}

#[enum_dispatch]
pub trait IActiveMessage {
    fn build_page<'a>(&'a mut self, ctx: &'a Context) -> BoxFuture<'a, Result<EmbedBuilder>>;

    fn build_components(&self) -> Vec<Component> {
        Vec::new()
    }

    fn handle_component(
        &mut self,
        component: &mut InteractionComponent,
    ) -> BoxFuture<'_, ComponentResult>;

    fn handle_modal(&mut self, _modal: &mut InteractionModal) -> BoxFuture<'_, Result<()>> {
        Box::pin(future::ready(Ok(())))
    }
}

pub enum ComponentResult {
    CreateModal(ModalBuilder),
    BuildPage,
    Err(Report),
}
