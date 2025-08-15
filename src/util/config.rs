use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use twilight_model::id::{
    marker::{GuildMarker, UserMarker},
    Id,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub title: String,
    pub discord_config: DiscordConfig,
    pub github_config: GithubConfig,
    pub issue_labels: IssueLabels,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub token: String,
    pub guild_id: u64,
    pub operator_id: Vec<u64>,
}

impl DiscordConfig {
    pub fn guild_id_as_marker(&self) -> Id<GuildMarker> {
        Id::new(self.guild_id)
    }

    pub fn operator_id_as_marker(&self) -> Vec<Id<UserMarker>> {
        self.operator_id.iter().map(|id| Id::new(*id)).collect()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GithubConfig {
    pub token: String,
    pub owner: String,
    pub repositories: HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IssueLabels {
    pub issue_types: Vec<String>,
    pub issue_priority: Vec<String>,
}
