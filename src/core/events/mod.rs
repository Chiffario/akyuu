use std::sync::Arc;

use eyre::Result;
use tracing::instrument::WithSubscriber;
use twilight_gateway::{
    error::{ReceiveMessageError, ReceiveMessageErrorType},
    Event, EventTypeFlags, Shard, StreamExt,
};

use self::interaction::handle_interaction;

use super::Context;

mod interaction;

pub async fn event_loop(ctx: Arc<Context>, shard: &mut Shard) {
    let flags = EventTypeFlags::GATEWAY_INVALIDATE_SESSION
        | EventTypeFlags::GATEWAY_RECONNECT
        | EventTypeFlags::INTERACTION_CREATE
        | EventTypeFlags::READY
        | EventTypeFlags::RESUMED;

    loop {
        let event = match shard.next_event(flags).await {
            Some(Ok(event)) => event,
            Some(Err(err)) => {
                warn!(?err, "Error: ");
                continue;
            }
            _ => continue,
        };

        let ctx = Arc::clone(&ctx);

        tokio::spawn(async move {
            if let Err(err) = handle_event(ctx, event).await {
                error!(?err, "Error while handling event");
            }
        });
    }
}

async fn handle_event(ctx: Arc<Context>, event: Event) -> Result<()> {
    match event {
        Event::GatewayInvalidateSession(true) => {
            info!("Gateway invalidated session but it's reconnectable")
        }
        Event::GatewayInvalidateSession(false) => {
            info!("Gateway invalidated session")
        }
        Event::GatewayReconnect => info!("Gateway requested shard to reconnect"),
        Event::InteractionCreate(e) => handle_interaction(ctx, e.0).await,
        Event::Ready(_) => info!("Shard is ready"),
        Event::Resumed => info!("Shard is resumed"),
        _ => {}
    }

    Ok(())
}
