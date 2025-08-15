#[macro_use]
extern crate eyre;

#[macro_use]
extern crate tracing;

use std::{
    fs::File,
    io::Read,
    path::PathBuf,
    sync::{Arc, LazyLock, OnceLock},
};

use clap::{parser::Values, Arg, Command};
use eyre::{Result, WrapErr};
use octocrab::OctocrabBuilder;
use tokio::signal;
use twilight_gateway::CloseFrame;
use twilight_http::Client;
use twilight_model::channel::message::AllowedMentions;
use util::config::{self, Project};

use crate::core::{commands::slash::INTERACTION_COMMANDS, event_loop, logging, Context};

mod active;
mod commands;
mod core;
mod util;

static CONFIG: OnceLock<Project> = OnceLock::new();

#[tokio::main]
async fn main() -> Result<()> {
    let command = Command::new("issue-bot").arg(
        Arg::new("config")
            .long("configuration")
            .short('c')
            .help("Configuration file"),
    );
    let matches = command.get_matches();
    let path = matches.get_one::<String>("config").unwrap();
    let path = PathBuf::from(path);
    let _log_worker_guard = logging::initialize();

    // let env_vars = EnvVars::read()?;
    let mut config_file = File::open(path).unwrap();
    let mut config_str = String::new();
    let _ = config_file.read_to_string(&mut config_str).unwrap();
    let config = ron::de::from_str::<Project>(&config_str).unwrap();

    println!("Bot initialized for project: {}", config.title);
    info!("Configuration: {:?}", config);
    CONFIG.get_or_init(|| ron::de::from_str::<Project>(&config_str).unwrap());
    // info!("Issue labels: {:?}", env_vars.issue_labels);

    let github = OctocrabBuilder::new()
        .personal_token(config.github_config.token.to_string())
        .build()
        .wrap_err("Failed to build github client")?;

    let mentions = AllowedMentions {
        replied_user: true,
        ..Default::default()
    };

    let http = Client::builder()
        .token(CONFIG.get().unwrap().discord_config.token.to_string())
        .remember_invalid_token(false)
        .default_allowed_mentions(mentions)
        .build();

    let current_user = http
        .current_user()
        .await
        .wrap_err("Failed to receive current user")?
        .model()
        .await
        .wrap_err("Failed to deserialize current user")?;

    let http = Arc::new(http);
    let mut shard =
        Context::create_shard(CONFIG.get().unwrap().discord_config.token.to_string(), None);

    let guild_id =
        twilight_model::id::Id::new(CONFIG.get().unwrap().discord_config.guild_id as u64);

    let ctx = Context {
        application_id: current_user.id.cast(),
        config,
        http,
        github,
        active_msgs: Default::default(),
    };

    INTERACTION_COMMANDS
        .register(&ctx.interaction(), guild_id)
        .await
        .wrap_err("Failed to register interaction commands")?;

    let ctx = Arc::new(ctx);

    tokio::select! {
        _ = event_loop(ctx, &mut shard) => warn!("Event loop ended"),
        res = signal::ctrl_c() => if let Err(err) = res {
            error!(?err, "Error while awaiting ctrl+c");
        } else {
            info!("Received Ctrl+C");
        },
    }

    if let Err(err) = shard.close(CloseFrame::NORMAL).await {
        warn!(?err, "Failed to close shard gracefully");
    }

    info!("Shutting down");

    Ok(())
}
