#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(dead_code)]

mod process;
mod types;

use std::{error::Error, fs::File, io::BufReader, panic, path::PathBuf, sync::OnceLock};

use clap::Parser;
use mongodb::sync::Client;
use process::parse_collections;
use serde_json::from_reader;
use tracing::{debug, error, warn};
use types::{Cli, Config};

static CONFIG: OnceLock<Config> = OnceLock::new();

fn main() {
    panic::set_hook(Box::new(|panic_info| {
        debug!("{:#?}", panic_info);
        error!("Operation has been canceled.");
    }));

    tracing_subscriber::fmt::init();

    let params = Cli::parse();

    let config = CONFIG.get_or_init(|| {
        File::open(
            params
                .config_file
                .unwrap_or_else(|| PathBuf::from("./config.json")),
        )
        .map_err(Box::from)
        .and_then(|file| from_reader(BufReader::new(file)).map_err(Box::from))
        .unwrap_or_else(|error: Box<dyn Error>| error_exit!("Error when processing config", error))
    });

    let db = Client::with_uri_str(&config.uri)
        .unwrap_or_else(|error| error_exit!("Error when processing config", error))
        .database(&config.database);

    let collections = if config.collections.is_empty() {
        db.list_collections(None, None).map_or_else(
            |error| error_exit!("Error when fetching collections", error),
            |collection| {
                collection
                    .into_iter()
                    .filter_map(|data| data.ok().map(|value| value.name))
                    .collect()
            },
        )
    } else {
        config.collections.clone()
    };

    parse_collections(&db, collections);
}
