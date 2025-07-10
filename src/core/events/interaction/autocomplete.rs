use std::{mem, sync::Arc};

use eyre::Result;

use crate::{core::Context, util::interaction::InteractionCommand};

pub async fn handle_autocomplete(_ctx: Arc<Context>, mut command: InteractionCommand) {
    let name = mem::take(&mut command.data.name);

    #[allow(unused, clippy::match_single_binding)]
    let res: Result<()> = match name.as_str() {
        _ => return error!(?name, "Unknown autocomplete command"),
    };

    #[allow(unreachable_code)]
    if let Err(err) = res {
        error!(?name, ?err, "Failed to process autocomplete");
    }
}
