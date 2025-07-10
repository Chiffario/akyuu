use std::sync::Arc;

use octocrab::Octocrab;
use twilight_gateway::{Config, EventTypeFlags, Intents, Shard, ShardId};
use twilight_http::{client::InteractionClient, Client};
use twilight_model::{
    gateway::{
        payload::outgoing::update_presence::UpdatePresencePayload,
        presence::{ActivityType, MinimalActivity, Status},
    },
    id::{marker::ApplicationMarker, Id},
};

use crate::{active::ActiveMessages, util::config::Project};

pub struct Context {
    pub application_id: Id<ApplicationMarker>,
    pub config: Project,
    // pub env_vars: EnvVars,.
    pub http: Arc<Client>,
    pub github: Octocrab,
    pub active_msgs: ActiveMessages,
}

impl Context {
    pub fn interaction(&self) -> InteractionClient<'_> {
        self.http.interaction(self.application_id)
    }

    pub fn create_shard(token: String, activity: Option<String>) -> Shard {
        let flags = EventTypeFlags::GATEWAY_INVALIDATE_SESSION
            | EventTypeFlags::GATEWAY_RECONNECT
            | EventTypeFlags::INTERACTION_CREATE
            | EventTypeFlags::READY
            | EventTypeFlags::RESUMED;

        let mut shard_config = Config::builder(token, Intents::empty()).event_types(flags);

        if let Some(activity) = activity {
            let activity = MinimalActivity {
                kind: ActivityType::Playing,
                name: activity,
                url: None,
            };

            let presence =
                UpdatePresencePayload::new([activity.into()], false, None, Status::Online).unwrap();

            shard_config = shard_config.presence(presence);
        }

        Shard::with_config(ShardId::ONE, shard_config.build())
    }
}
