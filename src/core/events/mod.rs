use std::sync::Arc;

use eyre::Result;
use twilight_gateway::{Event, Shard};

use self::interaction::handle_interaction;

use super::Context;

mod interaction;

pub async fn event_loop(ctx: Arc<Context>, shard: &mut Shard) {
    loop {
        let event = match shard.next_event().await {
            Ok(event) => event,
            Err(err) => {
                warn!(?err, "Error receiving event");

                if err.is_fatal() {
                    break;
                }

                continue;
            }
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
